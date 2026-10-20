//! AION — structural validator for the AION v0.0.1-1 language-definition format.
//!
//! This CLI reads an `.aion` file and reports whether it is *structurally*
//! valid according to the AION v0.0.1-1 grammar. It validates definitions only:
//! it does not generate languages or phonemes, invoke AI, produce ELANG output,
//! interpret linguistic meaning, or modify source files.
//!
//! Exit codes:
//! * `0` — valid definition,
//! * `1` — invalid definition,
//! * `2` — CLI usage error or file I/O failure.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser as ClapParser, Subcommand};

use aion_validator::diagnostic::Code;
use aion_validator::{check, diagnostic::Diagnostic};

/// Maximum accepted source size in bytes. Files larger than this are rejected
/// rather than silently truncated. Documented limit: 8 MiB.
const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;

/// AION definition validator.
#[derive(ClapParser, Debug)]
#[command(
    name = "aion",
    version,
    about = "Structural validator for AION v0.0.1-1 language definitions",
    long_about = "aion reads an .aion file and checks whether it is structurally \
valid according to the AION v0.0.1-1 grammar.\n\n\
It validates definitions only. It does not generate languages or phonemes, \
invoke AI, produce ELANG output, or modify source files."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate an AION definition file.
    Check {
        /// Path to the `.aion` file to validate.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { file } => run_check(&file),
    }
}

/// Run `aion check <file>` and return the process exit code.
fn run_check(path: &Path) -> ExitCode {
    let source = match read_source(path) {
        Ok(src) => src,
        Err(exit) => return exit,
    };

    let filename = path.display().to_string();

    match check(&source) {
        Ok(()) => {
            println!("Valid AION definition");
            ExitCode::from(0)
        }
        Err(diagnostics) => {
            render_all(&diagnostics, &filename, &source);
            eprintln!("Invalid AION definition");
            ExitCode::from(1)
        }
    }
}

/// Render every diagnostic to stderr, separated by a blank line.
fn render_all(diagnostics: &[Diagnostic], filename: &str, source: &str) {
    for diag in diagnostics {
        eprint!("{}", diag.render(filename, source));
        eprintln!();
    }
}

/// Read the source file, enforcing the size limit and reporting I/O and UTF-8
/// failures as usage errors (exit code 2).
fn read_source(path: &Path) -> Result<String, ExitCode> {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(err) => {
            eprintln!(
                "error[{}]: cannot access `{}`: {err}",
                Code::Input,
                path.display()
            );
            return Err(ExitCode::from(2));
        }
    };

    if !metadata.is_file() {
        eprintln!(
            "error[{}]: `{}` is not a regular file",
            Code::Input,
            path.display()
        );
        return Err(ExitCode::from(2));
    }

    if metadata.len() > MAX_FILE_SIZE {
        eprintln!(
            "error[{}]: `{}` is {} bytes, exceeding the {} byte limit",
            Code::Input,
            path.display(),
            metadata.len(),
            MAX_FILE_SIZE
        );
        return Err(ExitCode::from(2));
    }

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(err) => {
            eprintln!(
                "error[{}]: cannot read `{}`: {err}",
                Code::Input,
                path.display()
            );
            return Err(ExitCode::from(2));
        }
    };

    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(err) => {
            let offset = err.utf8_error().valid_up_to();
            eprintln!(
                "error[{}]: `{}` is not valid UTF-8 (first invalid byte at offset {})",
                Code::Input,
                path.display(),
                offset
            );
            Err(ExitCode::from(2))
        }
    }
}
