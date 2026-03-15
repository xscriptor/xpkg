# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in xpkg, **please do not open a
public issue.** Instead, report it privately:

**Email:** [x@xscriptor.com](mailto:x@xscriptor.com)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Response

- You will receive an acknowledgment within **48 hours**
- A fix will be developed and released as soon as possible
- You will be credited in the release notes (unless you prefer anonymity)

## Scope

The following are in scope:
- Code execution vulnerabilities in xpkg or xpkg-core
- Package signature bypass or verification flaws
- Path traversal during archive extraction
- Privilege escalation via fakeroot or build isolation
- Dependency confusion or supply chain issues

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest  | `main`|
