//! AION — structural validator library for the AION v0.0.1-1
//! language-definition format.
//!
//! This crate validates AION *definitions* only. It does not generate
//! languages or phonemes, invoke AI, produce ELANG output, interpret linguistic
//! meaning, or modify source files.
//!
//! The public surface is intentionally small:
//!
//! * [`parser::parse`] turns source text into a parse tree or a syntax
//!   [`diagnostic::Diagnostic`];
//! * [`validator::validate`] walks that tree and returns structural
//!   diagnostics;
//! * [`check`] is the convenience entry point that combines both.
//!
//! Rendering diagnostics and choosing process exit codes is the CLI's job.

pub mod diagnostic;
pub mod parser;
pub mod validator;

use diagnostic::Diagnostic;

/// Validate AION `source` text.
///
/// Returns `Ok(())` when the definition is structurally valid, or a non-empty
/// list of [`Diagnostic`]s describing the problems found. A syntax error is
/// returned as a single-element list.
pub fn check(source: &str) -> Result<(), Vec<Diagnostic>> {
    let diagnostics = match parser::parse(source) {
        Ok(mut pairs) => match pairs.next() {
            Some(document) => validator::validate(source, document),
            None => vec![Diagnostic::new(diagnostic::Code::Syntax, "empty document")],
        },
        Err(diag) => vec![diag],
    };

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str =
        "aion 0.0.1-1;\nlanguage t {\n    metadata {\n        description = \"d\";\n    }\n}\n";

    #[test]
    fn check_ok_for_valid_source() {
        assert!(check(VALID).is_ok());
    }

    #[test]
    fn check_err_aggregates_validator_diagnostics() {
        // Missing metadata -> at least one diagnostic.
        let src = "aion 0.0.1-1;\nlanguage t {\n}\n";
        let diags = check(src).expect_err("should be invalid");
        assert!(!diags.is_empty());
        assert!(diags
            .iter()
            .any(|d| d.code == diagnostic::Code::MissingRequired));
    }

    #[test]
    fn check_err_wraps_syntax_diagnostic_as_single_element() {
        // A parse failure returns exactly one syntax diagnostic.
        let diags = check("not aion at all").expect_err("syntax error");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, diagnostic::Code::Syntax);
    }
}
