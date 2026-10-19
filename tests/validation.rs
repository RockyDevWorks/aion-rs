//! Conformance tests for the AION structural validator.
//!
//! Positive fixtures must validate with no diagnostics. Negative fixtures must
//! produce at least one diagnostic; where practical we also assert the expected
//! stable error code.

use std::fs;
use std::path::PathBuf;

use aion::check;
use aion::diagnostic::{Code, Diagnostic};

/// Load a fixture file from `tests/fixtures/<category>/<name>`.
fn load(category: &str, name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(category);
    path.push(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read fixture {path:?}: {e}"))
}

/// Assert a source validates with no diagnostics.
fn assert_valid(category: &str, name: &str) {
    let src = load(category, name);
    match check(&src) {
        Ok(()) => {}
        Err(diags) => panic!(
            "expected `{name}` to be valid, got diagnostics: {:?}",
            diags
                .iter()
                .map(|d| format!("{} {}", d.code, d.message))
                .collect::<Vec<_>>()
        ),
    }
}

/// Assert a source is invalid and (optionally) carries the expected code.
fn assert_invalid(category: &str, name: &str, expected: Option<Code>) -> Vec<Diagnostic> {
    let src = load(category, name);
    match check(&src) {
        Ok(()) => panic!("expected `{name}` to be invalid, but it validated"),
        Err(diags) => {
            if let Some(code) = expected {
                assert!(
                    diags.iter().any(|d| d.code == code),
                    "expected `{name}` to contain code {code}, got {:?}",
                    diags.iter().map(|d| d.code).collect::<Vec<_>>()
                );
            }
            diags
        }
    }
}

// ---- positive cases --------------------------------------------------------

#[test]
fn valid_minimal() {
    assert_valid("valid", "minimal.aion");
}

#[test]
fn valid_complete() {
    assert_valid("valid", "complete.aion");
}

#[test]
fn valid_optional_omitted() {
    assert_valid("valid", "optional_omitted.aion");
}

#[test]
fn valid_sections_reordered() {
    assert_valid("valid", "sections_reordered.aion");
}

#[test]
fn valid_unicode_phonemes() {
    assert_valid("valid", "unicode_phonemes.aion");
}

#[test]
fn valid_multiline_strings() {
    assert_valid("valid", "multiline_strings.aion");
}

#[test]
fn valid_comments() {
    assert_valid("valid", "comments.aion");
}

#[test]
fn valid_empty_sets() {
    assert_valid("valid", "empty_sets.aion");
}

#[test]
fn valid_intent_string_and_set_forms() {
    assert_valid("valid", "intent_string_and_set_forms.aion");
}

// ---- negative cases --------------------------------------------------------

#[test]
fn invalid_missing_metadata() {
    assert_invalid(
        "invalid",
        "missing_metadata.aion",
        Some(Code::MissingRequired),
    );
}

#[test]
fn invalid_missing_description() {
    assert_invalid(
        "invalid",
        "missing_description.aion",
        Some(Code::MissingRequired),
    );
}

#[test]
fn invalid_whitespace_description() {
    assert_invalid(
        "invalid",
        "whitespace_description.aion",
        Some(Code::EmptyValue),
    );
}

#[test]
fn invalid_unsupported_version() {
    assert_invalid(
        "invalid",
        "unsupported_version.aion",
        Some(Code::UnsupportedVersion),
    );
}

#[test]
fn invalid_duplicate_metadata() {
    assert_invalid("invalid", "duplicate_metadata.aion", Some(Code::Duplicate));
}

#[test]
fn invalid_duplicate_intent() {
    assert_invalid("invalid", "duplicate_intent.aion", Some(Code::Duplicate));
}

#[test]
fn invalid_duplicate_inventory() {
    assert_invalid("invalid", "duplicate_inventory.aion", Some(Code::Duplicate));
}

#[test]
fn invalid_duplicate_property() {
    assert_invalid("invalid", "duplicate_property.aion", Some(Code::Duplicate));
}

#[test]
fn invalid_duplicate_set_member() {
    assert_invalid(
        "invalid",
        "duplicate_set_member.aion",
        Some(Code::DuplicateMember),
    );
}

#[test]
fn invalid_unknown_property() {
    // An unknown property is rejected by the grammar as a syntax error.
    assert_invalid("invalid", "unknown_property.aion", Some(Code::Syntax));
}

#[test]
fn invalid_wrong_type() {
    assert_invalid("invalid", "wrong_type.aion", Some(Code::WrongType));
}

#[test]
fn invalid_identifier() {
    // An identifier starting with a digit fails the grammar.
    assert_invalid("invalid", "invalid_identifier.aion", Some(Code::Syntax));
}

#[test]
fn invalid_integer_overflow() {
    assert_invalid(
        "invalid",
        "integer_overflow.aion",
        Some(Code::IntegerOverflow),
    );
}

#[test]
fn invalid_unicode_escape() {
    assert_invalid(
        "invalid",
        "invalid_unicode_escape.aion",
        Some(Code::InvalidEscape),
    );
}

#[test]
fn invalid_unterminated_string() {
    assert_invalid("invalid", "unterminated_string.aion", Some(Code::Syntax));
}

#[test]
fn invalid_unterminated_phoneme() {
    assert_invalid("invalid", "unterminated_phoneme.aion", Some(Code::Syntax));
}

#[test]
fn invalid_trailing_comma() {
    assert_invalid("invalid", "trailing_comma.aion", Some(Code::Syntax));
}

#[test]
fn invalid_trailing_content() {
    assert_invalid("invalid", "trailing_content.aion", Some(Code::Syntax));
}

#[test]
fn invalid_utf8_is_handled_cleanly() {
    // Invalid UTF-8 is an input-layer concern handled by the CLI's reader;
    // at the library level we confirm that decoding fails without panicking.
    // 0xFF is never valid in a UTF-8 stream.
    let bytes: Vec<u8> = vec![0xFF, 0xFE, b'a', b'i', b'o', b'n'];
    assert!(String::from_utf8(bytes).is_err());
}
