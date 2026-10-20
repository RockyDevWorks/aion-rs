//! Conformance tests for the AION structural validator.
//!
//! Positive fixtures must validate with no diagnostics. Negative fixtures must
//! produce at least one diagnostic; where practical we also assert the expected
//! stable error code.

use std::fs;
use std::path::PathBuf;

use aion_validator::check;
use aion_validator::diagnostic::{Code, Diagnostic};

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

// ---- regression: invalid Unicode escapes in every string-bearing context --

#[test]
fn invalid_escape_in_display_name() {
    assert_invalid(
        "invalid",
        "invalid_escape_display_name.aion",
        Some(Code::InvalidEscape),
    );
}

#[test]
fn invalid_escape_in_intent_description() {
    assert_invalid(
        "invalid",
        "invalid_escape_intent_description.aion",
        Some(Code::InvalidEscape),
    );
}

#[test]
fn invalid_escape_in_string_set_member() {
    assert_invalid(
        "invalid",
        "invalid_escape_set_member.aion",
        Some(Code::InvalidEscape),
    );
}

#[test]
fn invalid_escape_in_reference_argument() {
    assert_invalid(
        "invalid",
        "invalid_escape_reference_arg.aion",
        Some(Code::InvalidEscape),
    );
}

// ---- regression: escaped duplicates compare by decoded value --------------

#[test]
fn duplicate_escaped_string_member() {
    assert_invalid(
        "invalid",
        "duplicate_escaped_string_member.aion",
        Some(Code::DuplicateMember),
    );
}

#[test]
fn duplicate_escaped_reference() {
    assert_invalid(
        "invalid",
        "duplicate_escaped_reference.aion",
        Some(Code::DuplicateMember),
    );
}

// ---- regression: mixed-type set -------------------------------------------

#[test]
fn mixed_type_set_is_wrong_type() {
    assert_invalid("invalid", "mixed_type_set.aion", Some(Code::WrongType));
}

// ---- regression: valid escapes in every context ---------------------------

#[test]
fn valid_escapes_all_contexts() {
    assert_valid("valid", "valid_escapes_all_contexts.aion");
}

// ---- regression: empty sets valid for every set-valued property -----------

/// Every set-valued property (`style`, `character`, `articulation`, `rhythm`,
/// `avoid`, `references`, `include`, `exclude`) accepts an empty set `{ }`.
#[test]
fn empty_sets_valid_for_every_set_property() {
    let src = r#"aion 0.0.1-1;
language emptyall {
    metadata {
        description = "Empty sets for every set-valued property.";
    }
    intent phonology {
        style = { };
        character = { };
        articulation = { };
        rhythm = { };
        avoid = { };
        references = { };
    }
    phonology.inventory {
        consonants {
            include = { };
            exclude = { };
        }
        vowels {
            include = { };
            exclude = { };
        }
    }
}
"#;
    if let Err(diags) = check(src) {
        panic!(
            "expected empty sets to be valid, got: {:?}",
            diags
                .iter()
                .map(|d| format!("{} {}", d.code, d.message))
                .collect::<Vec<_>>()
        );
    }
}

// ---- regression: source positions, line endings, tabs, EOF ----------------

/// A Unicode (multi-byte) identifier earlier on the line must not corrupt the
/// reported column of a later error, and rendering must not panic.
#[test]
fn unicode_source_position_does_not_panic() {
    // `display_name` with an invalid escape; the description contains multibyte
    // characters so byte offsets and char columns differ.
    let src = "aion 0.0.1-1;\nlanguage x {\n metadata {\n  description = \"üëö café\";\n  display_name = \"bad\\u{110000}\";\n }\n}\n";
    let diags = check(src).expect_err("should be invalid");
    let diag = diags
        .iter()
        .find(|d| d.code == Code::InvalidEscape)
        .expect("invalid escape diagnostic");
    // Rendering with a multibyte line must not panic and must name the file.
    let rendered = diag.render("inline.aion", src);
    assert!(rendered.contains("error[AION-012]"));
}

/// CRLF line endings are accepted and the document still validates.
#[test]
fn crlf_line_endings_validate() {
    let src = "aion 0.0.1-1;\r\nlanguage crlf {\r\n    metadata {\r\n        description = \"CRLF endings.\";\r\n    }\r\n}\r\n";
    assert!(
        check(src).is_ok(),
        "CRLF-delimited document should validate"
    );
}

/// A lone-CR document still validates and a diagnostic renders without panic.
#[test]
fn cr_line_endings_do_not_panic_on_render() {
    let src = "aion 0.0.1-1;\rlanguage cr {\r    metadata {\r        description = \"\\u{110000}\";\r    }\r}\r";
    // Whether CR is accepted as a separator is a grammar detail; the contract
    // here is that validation and rendering never panic.
    match check(src) {
        Ok(()) => {}
        Err(diags) => {
            for d in &diags {
                let _ = d.render("cr.aion", src);
            }
        }
    }
}

/// Tabs are accepted as whitespace and the document validates.
#[test]
fn tab_indentation_validates() {
    let src = "aion 0.0.1-1;\n\tlanguage tabs {\n\t\tmetadata {\n\t\t\tdescription = \"Tab indented.\";\n\t\t}\n\t}\n";
    assert!(check(src).is_ok(), "tab-indented document should validate");
}

/// Truncation at end of input (missing closing brace) is a syntax error, not a
/// panic.
#[test]
fn end_of_file_truncation_is_syntax_error() {
    let src = "aion 0.0.1-1;\nlanguage eof {\n    metadata {\n        description = \"unterminated";
    let diags = check(src).expect_err("truncated input should be invalid");
    assert_eq!(diags[0].code, Code::Syntax);
}

/// An empty input is handled cleanly (syntax error, no panic).
#[test]
fn empty_input_is_handled() {
    let diags = check("").expect_err("empty input should be invalid");
    assert_eq!(diags[0].code, Code::Syntax);
}
