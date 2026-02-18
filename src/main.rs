#![warn(clippy::pedantic)]
// f64 casts for human-readable size display are intentional and bounded.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]

use anyhow::{Context as _, bail};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use human_bytes::human_bytes;
use pdf_size_shrinker::{Error as PdfError, Quality};
use pdf_size_shrinker::{ShrinkOptions, collect_pdfs, find_ghostscript, output_path, shrink_pdf};
use std::fs;
use std::path::PathBuf;

// ── CLI types ─────────────────────────────────────────────────────────────────

/// CLI mirror of [`Quality`] that derives [`ValueEnum`] for clap integration.
#[derive(Copy, Clone, Debug, ValueEnum)]
enum QualityArg {
    /// 72 dpi — smallest file, lowest quality (screen viewing only)
    Screen,
    /// 150 dpi — good quality, significantly smaller (e-readers / web) [default]
    Ebook,
    /// 300 dpi — high quality (desktop printing)
    Printer,
    /// 300 dpi — maximum quality, colour-preserving (professional print)
    Prepress,
}

impl From<QualityArg> for Quality {
    fn from(q: QualityArg) -> Self {
        match q {
            QualityArg::Screen => Self::Screen,
            QualityArg::Ebook => Self::Ebook,
            QualityArg::Printer => Self::Printer,
            QualityArg::Prepress => Self::Prepress,
        }
    }
}

/// Compress and reduce PDF file sizes using Ghostscript.
#[derive(Parser, Debug)]
#[command(name = "pdfshrinker", version, about, long_about = None)]
struct Cli {
    /// Input PDF file(s) or directory
    #[arg(required = true, value_name = "INPUT")]
    inputs: Vec<PathBuf>,

    /// Write compressed files to this directory instead of alongside the originals
    #[arg(short, long, value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Suffix appended to the output filename
    #[arg(short, long, value_name = "SUFFIX", default_value = "_compressed")]
    suffix: String,

    /// Compression quality preset
    #[arg(short, long, value_enum, default_value_t = QualityArg::Ebook)]
    quality: QualityArg,

    /// Recursively process subdirectories
    #[arg(short, long)]
    recursive: bool,

    /// Overwrite the original file in place (atomic: write temp then rename)
    #[arg(long)]
    in_place: bool,

    /// Show Ghostscript output
    #[arg(short, long)]
    verbose: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    if let Err(err) = run() {
        eprintln!("{} {err:#}", "error:".red().bold());
        std::process::exit(1);
    }
}

// ── Core runner ───────────────────────────────────────────────────────────────

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let gs = find_ghostscript().ok_or(PdfError::GhostscriptNotFound)?;

    if let Some(ref dir) = cli.output_dir {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create output directory '{}'", dir.display()))?;
    }

    let pdfs = collect_pdfs(&cli.inputs, cli.recursive);
    if pdfs.is_empty() {
        bail!("no PDF files found");
    }

    let opts = ShrinkOptions {
        gs_bin: &gs,
        quality: cli.quality.into(),
        verbose: cli.verbose,
    };

    let mut success: usize = 0;
    let mut failure: usize = 0;
    let mut total_saved: u64 = 0;

    for pdf in &pdfs {
        let out = if cli.in_place {
            pdf.with_extension("_pdfshrinker_tmp.pdf")
        } else {
            output_path(pdf, &cli.suffix, cli.output_dir.as_deref())
        };

        let dest_label = if cli.in_place {
            pdf.display().to_string()
        } else {
            out.display().to_string()
        };

        print!(
            "  {} {} \u{2192} {dest_label} ... ",
            "shrinking".cyan().bold(),
            pdf.display(),
        );

        let original_size = fs::metadata(pdf).map_or(0, |m| m.len());

        match shrink_pdf(&opts, pdf, &out) {
            Ok(()) => {
                let new_size = fs::metadata(&out).map_or(0, |m| m.len());

                if cli.in_place
                    && let Err(e) = fs::rename(&out, pdf)
                {
                    eprintln!("{} rename failed: {e}", "error:".red().bold());
                    let _ = fs::remove_file(&out);
                    failure += 1;
                    continue;
                }

                if new_size < original_size {
                    let saved = original_size - new_size;
                    total_saved = total_saved.saturating_add(saved);
                    let pct = (saved as f64 / original_size as f64) * 100.0;
                    println!(
                        "{} ({} \u{2192} {}, saved {} / {:.1}%)",
                        "done".green().bold(),
                        human_bytes(original_size as f64),
                        human_bytes(new_size as f64),
                        human_bytes(saved as f64),
                        pct,
                    );
                } else {
                    // Output did not shrink — discard it.
                    if !cli.in_place {
                        let _ = fs::remove_file(&out);
                    }
                    println!(
                        "{} ({} \u{2014} no reduction achieved; output discarded)",
                        "skip".yellow().bold(),
                        human_bytes(original_size as f64),
                    );
                }

                success += 1;
            }
            Err(e) => {
                println!("{}", "failed".red().bold());
                eprintln!("  {e}");
                let _ = fs::remove_file(&out);
                failure += 1;
            }
        }
    }

    println!();
    println!(
        "{} {} succeeded, {} failed \u{2014} total saved: {}",
        "summary:".bold(),
        success.to_string().green(),
        if failure > 0 {
            failure.to_string().red().to_string()
        } else {
            failure.to_string()
        },
        if total_saved > 0 {
            human_bytes(total_saved as f64).green().to_string()
        } else {
            human_bytes(0.0_f64).clone()
        },
    );

    if failure > 0 {
        std::process::exit(1);
    }

    Ok(())
}
