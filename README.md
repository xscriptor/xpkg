<h1 align="center">X Package Builder</h1>

> Developer tool for building, packaging, and maintaining packages for the X distribution and xpm.

## Overview

`xpkg` is the package building companion to [`xpm`](https://github.com/xscriptordev/xpm). It reads build recipes (`XBUILD` files), fetches sources, compiles software in a controlled environment, and produces `.xp` packages ready for installation with `xpm`. It maintains backward compatibility with Arch Linux `PKGBUILD` files.

Think of it as the `makepkg` equivalent for the X ecosystem — written in pure Rust, with modern tooling and safety guarantees.

### Key features

- **Pure Rust** — zero C dependencies, consistent with the xpm ecosystem
- **XBUILD format** — declarative TOML-based build recipes as a modern alternative to PKGBUILD shell scripts
- **PKGBUILD compatibility** — parse and build from Arch Linux PKGBUILD files
- **Native .xp output** — produces X Package archives (tar.zst) with `.PKGINFO` / `.BUILDINFO` / `.MTREE`
- **Fakeroot builds** — isolated build environment without requiring real root privileges
- **Package linting** — automated quality checks (dependencies, permissions, metadata, ELF analysis)
- **Repository tooling** — create and manage package repositories (`xpkg repo-add`, `xpkg repo-remove`)
- **Source management** — automatic download, verification, and extraction of source archives
- **Reproducible builds** — deterministic build metadata via `.BUILDINFO`

## Installation

```bash
git clone https://github.com/xscriptordev/xpkg.git
cd xpkg
cargo build --release
sudo cp target/release/xpkg /usr/local/bin/
```

## Usage

```bash
# Build a package from an XBUILD in the current directory
xpkg build

# Build from a PKGBUILD (Arch compatibility)
xpkg build --pkgbuild

# Build from a specific recipe file
xpkg build -f path/to/XBUILD

# Lint a package archive
xpkg lint <package.xp>

# Generate .SRCINFO equivalent from XBUILD
xpkg srcinfo

# Create a new XBUILD template
xpkg new <pkgname>

# Add a package to a repository database
xpkg repo-add <repo.db.tar.zst> <package.xp>

# Remove a package from a repository database
xpkg repo-remove <repo.db.tar.zst> <pkgname>

# Verify package integrity
xpkg verify <package.xp>

# Display package metadata
xpkg info <package.xp>
```

### Global flags

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Custom configuration file |
| `-v, --verbose` | Increase verbosity (-v, -vv, -vvv) |
| `--no-confirm` | Skip confirmation prompts |
| `--no-color` | Disable colored output |
| `-d, --builddir <PATH>` | Alternative build directory |
| `-o, --outdir <PATH>` | Output directory for built packages |

## XBUILD Format

`XBUILD` is a TOML-based build recipe format — declarative, type-safe, and easy to parse programmatically.

```toml
[package]
name = "example"
version = "1.0.0"
release = 1
description = "An example package"
url = "https://example.com"
license = ["GPL-3.0-or-later"]
arch = ["x86_64"]

[dependencies]
depends = ["glibc", "openssl"]
makedepends = ["cmake", "ninja"]
optdepends = ["docs: documentation files"]

[source]
urls = [
    "https://example.com/releases/example-1.0.0.tar.gz",
]
sha256sums = [
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
]

[build]
prepare = """
cd example-1.0.0
patch -p1 < ../fix-build.patch
"""

build = """
cd example-1.0.0
cmake -B build -G Ninja -DCMAKE_INSTALL_PREFIX=/usr
ninja -C build
"""

package = """
cd example-1.0.0
DESTDIR=$PKGDIR ninja -C build install
"""
```

## Configuration

Configuration file: `~/.config/xpkg/xpkg.conf` (TOML format).

```toml
[options]
builddir = "/tmp/xpkg-build"
outdir = "."
sign = false
sign_key = ""
strip_binaries = true
compress = "zstd"
compress_level = 19

[environment]
makeflags = "-j$(nproc)"
cflags = "-march=x86-64 -O2 -pipe"
cxxflags = "-march=x86-64 -O2 -pipe"

[lint]
enabled = true
strict = false
```

## Project structure

```text
xpkg/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── xpkg/                   # Binary crate (CLI frontend)
│   │   └── src/
│   │       ├── main.rs         # Entry point, logging, config, dispatch
│   │       └── cli.rs          # clap CLI definition
│   └── xpkg-core/              # Library crate (core logic)
│       └── src/
│           ├── lib.rs           # Module root
│           ├── config.rs        # Configuration parser
│           ├── error.rs         # Error types
│           ├── recipe/          # Build recipe parsing (XBUILD + PKGBUILD)
│           ├── builder/         # Build orchestration and fakeroot
│           ├── archive/         # Package archive creation (.xp)
│           ├── metadata/        # .PKGINFO, .BUILDINFO, .MTREE generation
│           ├── lint/            # Package linting framework
│           └── repo/            # Repository database management
├── docs/
│   ├── XBUILD.md               # XBUILD format specification
│   ├── CLI.md                   # CLI reference
│   └── LINTING.md              # Linting rules documentation
├── etc/
│   └── xpkg.conf.example       # Example configuration
└── ROADMAP.md                   # Development roadmap
```

## Relationship with xpm

`xpkg` and `xpm` form the two halves of the X packaging ecosystem:

| Tool | Role | Analogy |
|------|------|---------|
| `xpm` | Package manager — install, remove, upgrade, resolve dependencies | `pacman` |
| `xpkg` | Package builder — compile, package, lint, manage repositories | `makepkg` + `repo-add` + `namcap` |

`xpkg` produces `.xp` packages that `xpm` installs. They share the same package format specification and metadata structures, but are independent binaries with separate codebases.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full development roadmap.

| Version | Milestone |
|---|---|
| `v0.1.0` | Functional CLI with configuration |
| `v0.3.0` | Recipe parsing and source management |
| `v0.5.0` | Build engine, metadata generation, and archive creation |
| `v0.7.0` | Package linting framework |
| `v0.9.0` | Repository tooling and xpm integration |
| `v1.0.0` | Benchmarked, tested, production-ready |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
