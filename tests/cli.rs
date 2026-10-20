//! CLI integration tests for the `aion` binary.
//!
//! These exercise the process-level contract documented in `docs/cli.md`:
//!
//! * exit code `0` for a valid definition,
//! * exit code `1` for a structurally invalid definition,
//! * exit code `2` for CLI/usage and input errors (missing file, a path that
//!   is not a regular file, oversized input, and invalid UTF-8).
//!
//! The binary path is injected by Cargo via `CARGO_BIN_EXE_aion`, so no
//! hard-coded target directory is needed.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Absolute path to the compiled `aion` binary under test.
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aion")
}

/// A unique temp directory for this test process, created on first use.
fn temp_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("aion-cli-tests-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Write `contents` (raw bytes) to a uniquely named file and return its path.
fn write_fixture(name: &str, contents: &[u8]) -> PathBuf {
    let path = temp_dir().join(name);
    fs::write(&path, contents).expect("write fixture");
    path
}

/// Outcome of attempting to run the binary.
enum Run {
    /// The process ran: (exit_code, stdout, stderr).
    Ran(i32, String, String),
    /// The binary could not be spawned because the host forbids executing it
    /// (e.g. a Windows Application Control / WDAC policy, OS error 4551). The
    /// test should skip rather than fail: the assertion it would make is about
    /// the binary's behaviour, which is unobservable here. CI runners without
    /// such a policy exercise the real path.
    Blocked(String),
}

/// Run `aion check <path>`. Returns [`Run::Blocked`] when execution is barred
/// by host policy, otherwise [`Run::Ran`] with the captured result.
fn run_check(path: &std::path::Path) -> Run {
    match Command::new(bin()).arg("check").arg(path).output() {
        Ok(output) => Run::Ran(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ),
        Err(err) if is_exec_blocked(&err) => Run::Blocked(err.to_string()),
        Err(err) => panic!("spawn aion binary: {err}"),
    }
}

/// Whether a spawn error is a host execution-control block (WDAC / AppLocker).
fn is_exec_blocked(err: &std::io::Error) -> bool {
    // Windows ERROR_VIRUS_INFECTED / Application Control block is raw OS error
    // 4551. Match by raw code so the check is explicit and portable.
    err.raw_os_error() == Some(4551)
}

/// Helper: unwrap a [`Run`], or print a skip notice and return early.
macro_rules! ran_or_skip {
    ($run:expr) => {
        match $run {
            Run::Ran(code, stdout, stderr) => (code, stdout, stderr),
            Run::Blocked(why) => {
                eprintln!("skipping: cannot execute the aion binary on this host ({why})");
                return;
            }
        }
    };
}

const VALID_MINIMAL: &str = "aion 0.0.1-1;\n\
language aelith {\n\
    metadata {\n\
        description = \"An ancient, melodic constructed language.\";\n\
    }\n\
}\n";

// ---- exit code 0 -----------------------------------------------------------

#[test]
fn valid_file_exits_zero() {
    let path = write_fixture("valid.aion", VALID_MINIMAL.as_bytes());
    let (code, stdout, _stderr) = ran_or_skip!(run_check(&path));
    assert_eq!(code, 0, "valid file should exit 0");
    assert!(
        stdout.contains("Valid AION definition"),
        "stdout was: {stdout}"
    );
}

// ---- exit code 1 -----------------------------------------------------------

#[test]
fn invalid_definition_exits_one() {
    // Missing the required metadata block -> AION-003, exit 1.
    let src = "aion 0.0.1-1;\nlanguage x {\n}\n";
    let path = write_fixture("invalid.aion", src.as_bytes());
    let (code, _stdout, stderr) = ran_or_skip!(run_check(&path));
    assert_eq!(code, 1, "invalid definition should exit 1");
    assert!(
        stderr.contains("Invalid AION definition"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("error[AION-"), "stderr was: {stderr}");
}

#[test]
fn unsupported_version_exits_one() {
    let src =
        "aion 9.9.9-9;\nlanguage x {\n    metadata {\n        description = \"d\";\n    }\n}\n";
    let path = write_fixture("badversion.aion", src.as_bytes());
    let (code, _stdout, stderr) = ran_or_skip!(run_check(&path));
    assert_eq!(code, 1);
    assert!(stderr.contains("AION-002"), "stderr was: {stderr}");
}

// ---- exit code 2 -----------------------------------------------------------

#[test]
fn missing_file_exits_two() {
    let path = temp_dir().join("does-not-exist.aion");
    let (code, _stdout, stderr) = ran_or_skip!(run_check(&path));
    assert_eq!(code, 2, "missing file should exit 2");
    assert!(stderr.contains("AION-020"), "stderr was: {stderr}");
}

#[test]
fn directory_argument_exits_two() {
    // A directory is not a regular file -> input error, exit 2.
    let dir = temp_dir();
    let (code, _stdout, stderr) = ran_or_skip!(run_check(&dir));
    assert_eq!(code, 2, "directory argument should exit 2");
    assert!(stderr.contains("AION-020"), "stderr was: {stderr}");
}

#[test]
fn invalid_utf8_exits_two() {
    // 0xFF is never valid in a UTF-8 stream; reading must fail as an input
    // error (exit 2), not a validation error or a panic.
    let bytes: &[u8] = &[0xFF, 0xFE, b'a', b'i', b'o', b'n'];
    let path = write_fixture("badutf8.aion", bytes);
    let (code, _stdout, stderr) = ran_or_skip!(run_check(&path));
    assert_eq!(code, 2, "invalid UTF-8 should exit 2");
    assert!(stderr.contains("not valid UTF-8"), "stderr was: {stderr}");
}

#[test]
fn oversized_file_exits_two() {
    // The documented limit is 8 MiB; write one byte over it. The content need
    // not be valid AION — the size check happens before parsing.
    let size = 8 * 1024 * 1024 + 1;
    let big = vec![b'a'; size];
    let path = write_fixture("oversized.aion", &big);
    let run = run_check(&path);
    let _ = fs::remove_file(&path); // don't leave an 8 MiB file around
    let (code, _stdout, stderr) = ran_or_skip!(run);
    assert_eq!(code, 2, "oversized file should exit 2");
    assert!(
        stderr.contains("exceeding") && stderr.contains("limit"),
        "stderr was: {stderr}"
    );
}

#[test]
fn file_at_size_limit_is_read() {
    // A file exactly at the limit must be read (not rejected for size). It is
    // not valid AION, so it exits 1, proving the size gate passed.
    let size = 8 * 1024 * 1024;
    let mut content = VALID_MINIMAL.as_bytes().to_vec();
    // Pad with a trailing line comment up to exactly the limit so the file is
    // still parseable content-wise but reaches the boundary byte count.
    content.push(b'\n');
    let pad = size - content.len();
    content.extend(std::iter::repeat_n(b' ', pad));
    assert_eq!(content.len(), size);
    let path = write_fixture("atlimit.aion", &content);
    let run = run_check(&path);
    let _ = fs::remove_file(&path); // don't leave an 8 MiB file around
    let (code, _stdout, _stderr) = ran_or_skip!(run);
    // Trailing whitespace after the language block is a syntax error (exit 1),
    // which is fine: the point is that it was NOT rejected with exit 2.
    assert_ne!(
        code, 2,
        "a file exactly at the limit must not be size-rejected"
    );
}
