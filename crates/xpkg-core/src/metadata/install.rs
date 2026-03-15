//! `.INSTALL` script support.
//!
//! Handles optional installation hook scripts that run at specific points
//! during package installation, upgrade, or removal. These scripts are
//! packaged inside the `.xp` archive as the `.INSTALL` file.
//!
//! ## Supported hooks
//!
//! | Hook | When it runs |
//! |------|-------------|
//! | `pre_install` | Before files are installed (new install) |
//! | `post_install` | After files are installed (new install) |
//! | `pre_upgrade` | Before files are upgraded (version change) |
//! | `post_upgrade` | After files are upgraded (version change) |
//! | `pre_remove` | Before files are removed (uninstall) |
//! | `post_remove` | After files are removed (uninstall) |

use std::path::Path;

use crate::error::{XpkgError, XpkgResult};

/// Installation hook scripts for a package.
#[derive(Debug, Clone, Default)]
pub struct InstallScripts {
    pub pre_install: String,
    pub post_install: String,
    pub pre_upgrade: String,
    pub post_upgrade: String,
    pub pre_remove: String,
    pub post_remove: String,
}

impl InstallScripts {
    /// Returns true if all scripts are empty (no hooks defined).
    pub fn is_empty(&self) -> bool {
        self.pre_install.is_empty()
            && self.post_install.is_empty()
            && self.pre_upgrade.is_empty()
            && self.post_upgrade.is_empty()
            && self.pre_remove.is_empty()
            && self.post_remove.is_empty()
    }

    /// Load install scripts from a `.INSTALL` file.
    pub fn from_file(path: &Path) -> XpkgResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            XpkgError::Other(format!(
                "failed to read install script {}: {e}",
                path.display()
            ))
        })?;
        Ok(Self::parse(&content))
    }

    /// Parse install scripts from raw shell script content.
    ///
    /// The format uses shell functions:
    /// ```bash
    /// pre_install() {
    ///     echo "Installing..."
    /// }
    /// ```
    fn parse(content: &str) -> Self {
        Self {
            pre_install: extract_function(content, "pre_install"),
            post_install: extract_function(content, "post_install"),
            pre_upgrade: extract_function(content, "pre_upgrade"),
            post_upgrade: extract_function(content, "post_upgrade"),
            pre_remove: extract_function(content, "pre_remove"),
            post_remove: extract_function(content, "post_remove"),
        }
    }
}

/// Generate the `.INSTALL` file content from install scripts.
///
/// Returns `None` if no hooks are defined.
pub fn generate_install(scripts: &InstallScripts) -> Option<String> {
    if scripts.is_empty() {
        return None;
    }

    let mut out = String::new();

    write_function(&mut out, "pre_install", &scripts.pre_install);
    write_function(&mut out, "post_install", &scripts.post_install);
    write_function(&mut out, "pre_upgrade", &scripts.pre_upgrade);
    write_function(&mut out, "post_upgrade", &scripts.post_upgrade);
    write_function(&mut out, "pre_remove", &scripts.pre_remove);
    write_function(&mut out, "post_remove", &scripts.post_remove);

    Some(out)
}

/// Write a shell function to the output if it has a body.
fn write_function(out: &mut String, name: &str, body: &str) {
    if !body.is_empty() {
        out.push_str(&format!("{name}() {{\n{body}\n}}\n\n"));
    }
}

/// Extract a shell function body from script content.
///
/// Looks for `function_name() {` and extracts until the matching `}`.
fn extract_function(content: &str, name: &str) -> String {
    // Match patterns: "name() {" or "name () {"
    let patterns = [format!("{name}() {{"), format!("{name} () {{")];

    let start = patterns
        .iter()
        .filter_map(|pat| content.find(pat.as_str()))
        .min();

    let Some(start) = start else {
        return String::new();
    };

    // Find the opening brace.
    let after_brace = match content[start..].find('{') {
        Some(pos) => start + pos + 1,
        None => return String::new(),
    };

    // Track brace depth to find matching close.
    let mut depth = 1u32;
    let mut end = after_brace;
    for (i, ch) in content[after_brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = after_brace + i;
                    break;
                }
            }
            _ => {}
        }
    }

    let body = &content[after_brace..end];

    // Trim leading/trailing whitespace from each line, but preserve indentation structure.
    body.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_scripts_is_empty() {
        let scripts = InstallScripts::default();
        assert!(scripts.is_empty());
    }

    #[test]
    fn test_non_empty_scripts() {
        let scripts = InstallScripts {
            post_install: "echo done".into(),
            ..Default::default()
        };
        assert!(!scripts.is_empty());
    }

    #[test]
    fn test_generate_install_none_when_empty() {
        let scripts = InstallScripts::default();
        assert!(generate_install(&scripts).is_none());
    }

    #[test]
    fn test_generate_install_produces_functions() {
        let scripts = InstallScripts {
            pre_install: "echo 'pre-install'".into(),
            post_install: "echo 'post-install'".into(),
            ..Default::default()
        };

        let content = generate_install(&scripts).unwrap();
        assert!(content.contains("pre_install() {\n"));
        assert!(content.contains("post_install() {\n"));
        assert!(!content.contains("pre_upgrade"));
    }

    #[test]
    fn test_parse_install_script() {
        let content = r#"
pre_install() {
    echo "Installing..."
}

post_install() {
    ldconfig
}
"#;
        let scripts = InstallScripts::parse(content);
        assert_eq!(scripts.pre_install, "echo \"Installing...\"");
        assert_eq!(scripts.post_install, "ldconfig");
        assert!(scripts.pre_upgrade.is_empty());
    }

    #[test]
    fn test_parse_nested_braces() {
        let content = r#"
post_install() {
    if [ -f /etc/config ]; then
        echo "found"
    fi
}
"#;
        let scripts = InstallScripts::parse(content);
        assert!(scripts.post_install.contains("if [ -f /etc/config ]"));
        assert!(scripts.post_install.contains("fi"));
    }

    #[test]
    fn test_roundtrip_generate_parse() {
        let original = InstallScripts {
            pre_install: "echo 'pre'".into(),
            post_remove: "ldconfig".into(),
            ..Default::default()
        };

        let generated = generate_install(&original).unwrap();
        let parsed = InstallScripts::parse(&generated);

        assert_eq!(parsed.pre_install, "echo 'pre'");
        assert_eq!(parsed.post_remove, "ldconfig");
        assert!(parsed.post_install.is_empty());
    }
}
