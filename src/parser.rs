//! Parsing layer: wraps the Pest-generated parser.
//!
//! This module exposes a single clean entry point, [`parse`], which accepts
//! source text and returns either the root parse pair or a structured
//! [`Diagnostic`]. It never prints and never panics on malformed input.

use crate::diagnostic::{Code, Diagnostic, Span};
use pest::error::{ErrorVariant, InputLocation, LineColLocation};
use pest::Parser;
use pest_derive::Parser;

/// The Pest-generated parser for the AION grammar.
///
/// The grammar at `grammar/aion.pest` is the authoritative definition.
#[derive(Parser)]
#[grammar = "../grammar/aion.pest"]
pub struct AionParser;

/// The grammar rules, re-exported for the validator.
pub use Rule as GrammarRule;

/// Parse `source` into the root `document` pair.
///
/// On success returns the token tree owned by `pest`. On failure returns a
/// single [`Diagnostic`] with code [`Code::Syntax`] describing the first
/// syntax error, including its resolved span.
pub fn parse(source: &str) -> Result<pest::iterators::Pairs<'_, Rule>, Diagnostic> {
    match AionParser::parse(Rule::document, source) {
        Ok(pairs) => Ok(pairs),
        Err(err) => Err(syntax_diagnostic(source, err)),
    }
}

/// Convert a Pest parse error into an AION syntax diagnostic.
fn syntax_diagnostic(source: &str, err: pest::error::Error<Rule>) -> Diagnostic {
    let (start, end) = match err.location {
        InputLocation::Pos(p) => (p, (p + 1).min(source.len())),
        InputLocation::Span((s, e)) => (s, e),
    };
    let span = Span::from_offsets(source, start, end);

    let message = match &err.variant {
        ErrorVariant::ParsingError {
            positives,
            negatives,
        } => describe_parsing_error(positives, negatives),
        ErrorVariant::CustomError { message } => message.clone(),
    };

    // Derive a friendlier line/col even if the byte span was clamped.
    let (line, column) = match err.line_col {
        LineColLocation::Pos((l, c)) => (l, c),
        LineColLocation::Span((l, c), _) => (l, c),
    };
    let span = Span {
        line,
        column,
        ..span
    };

    Diagnostic::new(Code::Syntax, "syntax error")
        .with_span(span)
        .with_note(message)
}

/// Produce a human-readable description of expected/unexpected rules.
fn describe_parsing_error(positives: &[Rule], negatives: &[Rule]) -> String {
    let fmt_list = |rules: &[Rule]| rules.iter().map(rule_label).collect::<Vec<_>>().join(", ");

    match (positives.is_empty(), negatives.is_empty()) {
        (false, true) => format!("expected {}", fmt_list(positives)),
        (true, false) => format!("unexpected {}", fmt_list(negatives)),
        (false, false) => format!(
            "expected {}; unexpected {}",
            fmt_list(positives),
            fmt_list(negatives)
        ),
        (true, true) => "malformed input".to_string(),
    }
}

/// Map grammar rules to readable labels for diagnostics.
fn rule_label(rule: &Rule) -> String {
    let s = match rule {
        Rule::document => "a complete AION document",
        Rule::version_declaration => "a version declaration",
        Rule::version => "a version (e.g. 0.0.1-1)",
        Rule::language_declaration => "a language declaration",
        Rule::language_section => "a language section",
        Rule::metadata_block => "a metadata block",
        Rule::intent_phonology_block => "an `intent phonology` block",
        Rule::phonology_inventory_block => "a `phonology.inventory` block",
        Rule::string => "a string",
        Rule::quoted_string => "a quoted string",
        Rule::multiline_string => "a multiline string",
        Rule::phoneme => "a phoneme literal",
        Rule::identifier => "an identifier",
        Rule::integer => "an integer",
        Rule::reference => "a reference",
        Rule::EOI => "end of input",
        other => return format!("{other:?}"),
    };
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"aion 0.0.1-1;
language aelith {
    metadata {
        description = "An ancient, melodic constructed language.";
    }
}
"#;

    #[test]
    fn keyword_rejects_identifier_continuation() {
        // `aion0` must not be read as the `aion` keyword.
        assert!(AionParser::parse(Rule::version_declaration, "aion0.0.1-1;").is_err());
        // but `aion 0.0.1-1;` must parse.
        assert!(AionParser::parse(Rule::version_declaration, "aion 0.0.1-1;").is_ok());
    }

    #[test]
    fn parses_minimal_document() {
        assert!(parse(MINIMAL).is_ok());
    }

    #[test]
    fn rejects_trailing_content() {
        let src = format!("{MINIMAL}\nextra");
        let err = parse(&src).unwrap_err();
        assert_eq!(err.code, Code::Syntax);
        assert!(err.span.is_some());
    }

    #[test]
    fn rejects_unterminated_string_without_panic() {
        let src = "aion 0.0.1-1;\nlanguage x {\n metadata { description = \"oops;\n}\n}";
        let err = parse(src).unwrap_err();
        assert_eq!(err.code, Code::Syntax);
    }
}
