# pdfshrinker

[![CI](https://github.com/bzon/pdf-size-shrinker/actions/workflows/ci.yml/badge.svg)](https://github.com/bzon/pdf-size-shrinker/actions/workflows/ci.yml)
[![Release](https://github.com/bzon/pdf-size-shrinker/actions/workflows/release.yml/badge.svg)](https://github.com/bzon/pdf-size-shrinker/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**pdfshrinker** is a fast, cross-platform command-line tool that compresses and reduces PDF file sizes.
It wraps [Ghostscript](https://www.ghostscript.com/) to re-encode images, strip redundant data, and
apply flate compression — achieving up to **92% size reduction** on image-heavy PDFs.

Built with Rust. Pre-built binaries available for macOS, Linux, and Windows — no Rust toolchain required.

## Features

- Compress a single PDF, a list of files, or an entire directory tree
- Four quality presets: `screen` (72 dpi) → `ebook` (150 dpi) → `printer` / `prepress` (300 dpi)
- In-place mode — overwrites the original file atomically
- Skips files where compression would increase size
- Colored terminal output with per-file stats and a final summary
- Runs on macOS (Intel + Apple Silicon), Linux (x86-64 + ARM64), and Windows

## Prerequisites

Ghostscript must be installed and on your `PATH`.

| Platform | Install |
|----------|---------|
| macOS | `brew install ghostscript` |
| Ubuntu / Debian | `sudo apt-get install ghostscript` |
| Fedora / RHEL | `sudo dnf install ghostscript` |
| Windows | [ghostscript.com/download](https://www.ghostscript.com/download/gsdnld.html) |

## Install

### macOS & Linux — shell script (recommended)

Installs the latest pre-built binary to `/usr/local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/bzon/pdf-size-shrinker/main/install.sh | bash
```

Custom install directory:

```bash
INSTALL_DIR=~/.local/bin curl -fsSL https://raw.githubusercontent.com/bzon/pdf-size-shrinker/main/install.sh | bash
```

### macOS — Homebrew tap

After setting up the tap repo (see [Homebrew tap setup](#homebrew-tap-setup)):

```bash
brew tap bzon/pdfshrinker
brew install pdfshrinker
```

### Download a pre-built binary

Grab the archive for your platform from the [Releases page](https://github.com/bzon/pdf-size-shrinker/releases),
extract, and move `pdfshrinker` (or `pdfshrinker.exe`) to a directory on your `PATH`.

| Platform | Archive |
|----------|---------|
| macOS Apple Silicon | `pdfshrinker-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `pdfshrinker-*-x86_64-apple-darwin.tar.gz` |
| Linux x86-64 | `pdfshrinker-*-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `pdfshrinker-*-aarch64-unknown-linux-gnu.tar.gz` |
| Windows x86-64 | `pdfshrinker-*-x86_64-pc-windows-msvc.zip` |

### Install with Cargo

```bash
cargo install --git https://github.com/bzon/pdf-size-shrinker
```

## Quick start

```bash
# Compress a single PDF (outputs invoice_compressed.pdf)
pdfshrinker invoice.pdf

# Maximum compression
pdfshrinker --quality screen large-report.pdf

# Overwrite the original file
pdfshrinker --in-place invoice.pdf

# Compress every PDF in a folder
pdfshrinker ./documents/

# Recurse into subdirectories
pdfshrinker --recursive ./documents/
```

Example output:

```
  shrinking invoice.pdf → invoice_compressed.pdf ... done (29 MiB → 2.3 MiB, saved 26.7 MiB / 92.0%)

summary: 1 succeeded, 0 failed — total saved: 26.7 MiB
```

## Usage

```
pdfshrinker [OPTIONS] <INPUT>...
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-q, --quality <QUALITY>` | `ebook` | Quality preset (see table below) |
| `-o, --output-dir <DIR>` | same dir as input | Write output files to this directory |
| `-s, --suffix <SUFFIX>` | `_compressed` | Suffix appended to output filename |
| `-r, --recursive` | off | Recurse into subdirectories |
| `--in-place` | off | Overwrite the original file |
| `-v, --verbose` | off | Show Ghostscript output |

### Quality presets

| Preset | DPI | Best for |
|--------|-----|----------|
| `screen` | 72 | Smallest possible size, screen-only viewing |
| `ebook` | 150 | Good quality + significant size reduction *(default)* |
| `printer` | 300 | High quality for desktop printing |
| `prepress` | 300 | Maximum quality, color-preserving for professional print |

## How it works

`pdfshrinker` invokes Ghostscript's `pdfwrite` device with `-dPDFSETTINGS`, which
re-encodes embedded images at a lower DPI, removes unused resources, and applies
flate compression. No native Rust PDF parsing is involved — Ghostscript does the
heavy lifting, ensuring broad compatibility with any valid PDF.

If the compressed output is not smaller than the original, the output file is
discarded automatically and the original is left unchanged.

## Homebrew tap setup

To publish `pdfshrinker` via Homebrew after your first release:

1. Create a new GitHub repository named **`homebrew-pdfshrinker`** under your account.
2. Copy [`Formula/pdfshrinker.rb`](Formula/pdfshrinker.rb) from this repo into `Formula/pdfshrinker.rb` in the tap repo.
3. Replace the `sha256` placeholders with the actual checksums from `checksums.txt` in the release.
4. Users can then install with:
   ```bash
   brew tap bzon/pdfshrinker
   brew install pdfshrinker
   ```

## Contributing

Pull requests are welcome. Please run the following before submitting:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## License

[MIT](LICENSE)
