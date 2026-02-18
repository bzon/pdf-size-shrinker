#![warn(clippy::pedantic)]
#![warn(missing_docs)]
//! Core PDF shrinking logic.
//!
//! This crate drives [Ghostscript](https://www.ghostscript.com/) to recompress
//! embedded images, strip unused resources, and apply flate compression,
//! achieving significant size reductions on image-heavy PDFs.
//!
//! # Quick start
//!
//! ```no_run
//! use pdf_size_shrinker::{find_ghostscript, shrink_pdf, Quality, ShrinkOptions};
//! use std::path::Path;
//!
//! let gs = find_ghostscript().expect("Ghostscript not found");
//! let opts = ShrinkOptions { gs_bin: &gs, quality: Quality::Ebook, verbose: false };
//! shrink_pdf(&opts, Path::new("input.pdf"), Path::new("output.pdf")).unwrap();
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use walkdir::WalkDir;

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors that can occur during PDF collection or shrinking.
#[derive(Debug, Error)]
pub enum Error {
    /// Ghostscript executable was not found on the system `PATH`.
    #[error(
        "Ghostscript not found. Install it with:\n  \
         macOS:   brew install ghostscript\n  \
         Ubuntu:  sudo apt-get install ghostscript\n  \
         Windows: https://www.ghostscript.com/download/gsdnld.html"
    )]
    GhostscriptNotFound,

    /// Ghostscript was spawned but exited with a non-zero status.
    #[error("Ghostscript failed: {0}")]
    GhostscriptFailed(String),

    /// Ghostscript could not be spawned (e.g. missing execute permission).
    #[error("Failed to spawn Ghostscript: {0}")]
    Spawn(#[source] std::io::Error),

    /// An I/O error at a specific filesystem path.
    #[error("I/O error at '{path}': {source}")]
    Io {
        /// The path that triggered the error.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Convenience `Result` alias for this crate's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

// ── Quality ───────────────────────────────────────────────────────────────────

/// Compression quality preset, directly mapped to Ghostscript's
/// `-dPDFSETTINGS` option.
///
/// Lower presets produce smaller files at the cost of image fidelity.
/// For most documents [`Quality::Ebook`] is the best trade-off.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Quality {
    /// 72 dpi — smallest file, lowest quality (on-screen viewing only).
    Screen,
    /// 150 dpi — good quality, significantly smaller (e-readers / web).
    Ebook,
    /// 300 dpi — high quality (desktop printing).
    Printer,
    /// 300 dpi — maximum quality, colour-preserving (professional print).
    Prepress,
}

impl Quality {
    /// Returns the Ghostscript `-dPDFSETTINGS` argument value for this preset.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdf_size_shrinker::Quality;
    ///
    /// assert_eq!(Quality::Screen.gs_setting(), "/screen");
    /// assert_eq!(Quality::Ebook.gs_setting(), "/ebook");
    /// assert_eq!(Quality::Printer.gs_setting(), "/printer");
    /// assert_eq!(Quality::Prepress.gs_setting(), "/prepress");
    /// ```
    #[must_use]
    pub fn gs_setting(self) -> &'static str {
        match self {
            Self::Screen => "/screen",
            Self::Ebook => "/ebook",
            Self::Printer => "/printer",
            Self::Prepress => "/prepress",
        }
    }
}

// ── ShrinkOptions ─────────────────────────────────────────────────────────────

/// Configuration passed to [`shrink_pdf`].
#[derive(Debug)]
pub struct ShrinkOptions<'a> {
    /// Path or name of the Ghostscript executable (e.g. `"gs"` or `"/usr/bin/gs"`).
    pub gs_bin: &'a str,
    /// Quality preset controlling the `-dPDFSETTINGS` Ghostscript flag.
    pub quality: Quality,
    /// When `true`, Ghostscript's stdout is forwarded to the caller's stdout.
    pub verbose: bool,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Searches `PATH` for a usable Ghostscript executable.
///
/// Checks `gs`, `gswin64c`, and `gswin32c` in order and returns the name of
/// the first one that responds successfully to `--version`.
/// Returns `None` when Ghostscript is not available.
///
/// # Examples
///
/// ```no_run
/// use pdf_size_shrinker::find_ghostscript;
///
/// match find_ghostscript() {
///     Some(gs) => println!("Found Ghostscript: {gs}"),
///     None => eprintln!("Ghostscript not installed"),
/// }
/// ```
#[must_use]
pub fn find_ghostscript() -> Option<String> {
    ["gs", "gswin64c", "gswin32c"].iter().find_map(|&name| {
        Command::new(name)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|_| name.to_owned())
    })
}

/// Compresses `input` and writes the result to `output` using Ghostscript.
///
/// On success the `output` file will exist and contain the compressed PDF.
/// On any error the caller is responsible for cleaning up a partial `output`
/// file (if one was created).
///
/// # Errors
///
/// - [`Error::Spawn`] — Ghostscript could not be launched.
/// - [`Error::GhostscriptFailed`] — Ghostscript exited with a non-zero code.
///
/// # Examples
///
/// ```no_run
/// use pdf_size_shrinker::{find_ghostscript, shrink_pdf, Quality, ShrinkOptions};
/// use std::path::Path;
///
/// let gs = find_ghostscript().unwrap();
/// let opts = ShrinkOptions { gs_bin: &gs, quality: Quality::Ebook, verbose: false };
/// shrink_pdf(&opts, Path::new("in.pdf"), Path::new("out.pdf")).unwrap();
/// ```
pub fn shrink_pdf(opts: &ShrinkOptions<'_>, input: &Path, output: &Path) -> Result<()> {
    let mut cmd = Command::new(opts.gs_bin);
    cmd.arg("-sDEVICE=pdfwrite")
        .arg("-dCompatibilityLevel=1.4")
        .arg(format!("-dPDFSETTINGS={}", opts.quality.gs_setting()))
        .arg("-dNOPAUSE")
        .arg("-dBATCH")
        .arg("-dSAFER");

    if !opts.verbose {
        cmd.arg("-dQUIET");
    }

    cmd.arg(format!("-sOutputFile={}", output.display()))
        .arg(input);

    let result = cmd.output().map_err(Error::Spawn)?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(Error::GhostscriptFailed(stderr.into_owned()));
    }

    if opts.verbose && !result.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&result.stdout));
    }

    Ok(())
}

/// Computes the output path for a shrunk PDF.
///
/// When `output_dir` is `Some`, the file is placed in that directory.
/// Otherwise the compressed file is placed alongside `input`.
/// The filename is `<original-stem><suffix>.pdf`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use pdf_size_shrinker::output_path;
///
/// // Default: same directory as input.
/// let p = output_path(Path::new("/docs/report.pdf"), "_compressed", None);
/// assert_eq!(p, Path::new("/docs/report_compressed.pdf"));
///
/// // Custom output directory.
/// let p = output_path(Path::new("/docs/report.pdf"), "_compressed", Some(Path::new("/out")));
/// assert_eq!(p, Path::new("/out/report_compressed.pdf"));
/// ```
#[must_use]
pub fn output_path(input: &Path, suffix: &str, output_dir: Option<&Path>) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let name = format!("{stem}{suffix}.pdf");
    output_dir.map_or_else(|| input.with_file_name(&name), |dir| dir.join(&name))
}

/// Collects PDF files from a mixed list of file and directory paths.
///
/// - Plain files that do not end in `.pdf` (case-insensitive) are skipped with
///   a warning printed to stderr.
/// - Directories are walked one level deep unless `recursive` is `true`.
/// - Paths that do not exist produce a warning on stderr and are skipped.
///
/// Returns every matched PDF path in the order they were encountered.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use pdf_size_shrinker::collect_pdfs;
///
/// let pdfs = collect_pdfs(&[PathBuf::from("./invoices")], true);
/// println!("found {} PDFs", pdfs.len());
/// ```
#[must_use]
pub fn collect_pdfs(inputs: &[PathBuf], recursive: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for input in inputs {
        if input.is_file() {
            if is_pdf(input) {
                files.push(input.clone());
            } else {
                eprintln!("warn: skipping non-PDF file: {}", input.display());
            }
        } else if input.is_dir() {
            let max_depth = if recursive { usize::MAX } else { 1 };
            for entry in WalkDir::new(input)
                .max_depth(max_depth)
                .into_iter()
                .filter_map(std::result::Result::ok)
            {
                let path = entry.into_path();
                if path.is_file() && is_pdf(&path) {
                    files.push(path);
                }
            }
        } else {
            eprintln!("error: path not found: {}", input.display());
        }
    }

    files
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Returns `true` if `path` has a `.pdf` extension (case-insensitive).
#[inline]
fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Quality ──────────────────────────────────────────────────────────────────

    #[test]
    fn quality_gs_settings_are_correct() {
        assert_eq!(Quality::Screen.gs_setting(), "/screen");
        assert_eq!(Quality::Ebook.gs_setting(), "/ebook");
        assert_eq!(Quality::Printer.gs_setting(), "/printer");
        assert_eq!(Quality::Prepress.gs_setting(), "/prepress");
    }

    // output_path ──────────────────────────────────────────────────────────────

    #[test]
    fn output_path_same_directory() {
        let result = output_path(Path::new("/docs/report.pdf"), "_compressed", None);
        assert_eq!(result, Path::new("/docs/report_compressed.pdf"));
    }

    #[test]
    fn output_path_into_custom_directory() {
        let result = output_path(
            Path::new("/docs/report.pdf"),
            "_compressed",
            Some(Path::new("/out")),
        );
        assert_eq!(result, Path::new("/out/report_compressed.pdf"));
    }

    #[test]
    fn output_path_custom_suffix() {
        let result = output_path(Path::new("/a/b.pdf"), "_min", None);
        assert_eq!(result, Path::new("/a/b_min.pdf"));
    }

    #[test]
    fn output_path_preserves_stem_with_dots() {
        let result = output_path(Path::new("/a/report.v2.pdf"), "_small", None);
        assert_eq!(result, Path::new("/a/report.v2_small.pdf"));
    }

    // is_pdf ───────────────────────────────────────────────────────────────────

    #[test]
    fn is_pdf_matches_lowercase_extension() {
        assert!(is_pdf(Path::new("file.pdf")));
    }

    #[test]
    fn is_pdf_matches_uppercase_extension() {
        assert!(is_pdf(Path::new("file.PDF")));
    }

    #[test]
    fn is_pdf_matches_mixed_case_extension() {
        assert!(is_pdf(Path::new("file.Pdf")));
    }

    #[test]
    fn is_pdf_rejects_other_extensions() {
        assert!(!is_pdf(Path::new("file.docx")));
        assert!(!is_pdf(Path::new("file.txt")));
    }

    #[test]
    fn is_pdf_rejects_no_extension() {
        assert!(!is_pdf(Path::new("file")));
    }

    // collect_pdfs ─────────────────────────────────────────────────────────────

    #[test]
    fn collect_pdfs_skips_nonexistent_paths() {
        let result = collect_pdfs(&[PathBuf::from("/nonexistent/ghost.pdf")], false);
        assert!(result.is_empty());
    }

    #[test]
    fn collect_pdfs_skips_non_pdf_files() {
        // Use this source file which definitely exists but is not a PDF.
        let result = collect_pdfs(&[PathBuf::from(file!())], false);
        assert!(result.is_empty());
    }
}
