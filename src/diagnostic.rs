//! Structured diagnostics for AION validation.
//!
//! A [`Diagnostic`] carries a stable error code, a human-readable message, the
//! source location (filename, line, column, and a byte span), and an optional
//! explanatory note. The validator and parser produce `Diagnostic` values;
//! rendering to text and exit-code selection is the CLI's responsibility.

use std::fmt;

/// Stable AION diagnostic codes.
///
/// Codes are stable identifiers rendered as `AION-NNN`. They are intentionally
/// coarse-grained: one code per structural rule class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// Syntax error reported by the grammar (malformed/unterminated tokens,
    /// unexpected trailing content, invalid identifiers, etc.).
    Syntax,
    /// Unsupported version value (only `0.0.1-1` is accepted).
    UnsupportedVersion,
    /// A required element is missing (e.g. metadata block or description).
    MissingRequired,
    /// A block or property appears more than once where only one is allowed.
    Duplicate,
    /// A value has the wrong type for its property.
    WrongType,
    /// An integer is outside the representable u32 range.
    IntegerOverflow,
    /// A set contains duplicate members.
    DuplicateMember,
    /// A string/required value is empty or whitespace-only where content is
    /// required.
    EmptyValue,
    /// A unicode escape does not denote a valid Unicode scalar value.
    InvalidEscape,
    /// Input/limit error (file too large, invalid UTF-8 surfaced as a note).
    Input,
}

impl Code {
    /// The numeric portion of the stable error code.
    pub const fn number(self) -> u16 {
        match self {
            Code::Syntax => 1,
            Code::UnsupportedVersion => 2,
            Code::MissingRequired => 3,
            Code::Duplicate => 4,
            Code::WrongType => 8,
            Code::IntegerOverflow => 9,
            Code::DuplicateMember => 10,
            Code::EmptyValue => 11,
            Code::InvalidEscape => 12,
            Code::Input => 20,
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AION-{:03}", self.number())
    }
}

/// A resolved source location: 1-based line and column plus the byte span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the start of the span (0-based).
    pub start: usize,
    /// Byte offset of the end of the span (exclusive, 0-based).
    pub end: usize,
    /// 1-based line number of the span start.
    pub line: usize,
    /// 1-based column number of the span start.
    pub column: usize,
}

impl Span {
    /// Build a span from byte offsets, resolving line/column against `source`.
    pub fn from_offsets(source: &str, start: usize, end: usize) -> Span {
        let (line, column) = line_col(source, start);
        Span {
            start,
            end,
            line,
            column,
        }
    }
}

/// Compute a 1-based (line, column) for a byte offset within `source`.
///
/// Columns are counted in Unicode scalar values (chars), matching the way
/// editors typically report columns.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// A single validation or parse diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Stable error code.
    pub code: Code,
    /// Human-readable one-line message.
    pub message: String,
    /// Source location, when known.
    pub span: Option<Span>,
    /// Optional explanatory note (the `= ...` line in rendered output).
    pub note: Option<String>,
}

impl Diagnostic {
    /// Create a diagnostic with a code and message only.
    pub fn new(code: Code, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            code,
            message: message.into(),
            span: None,
            note: None,
        }
    }

    /// Attach a source span.
    pub fn with_span(mut self, span: Span) -> Diagnostic {
        self.span = Some(span);
        self
    }

    /// Attach an explanatory note.
    pub fn with_note(mut self, note: impl Into<String>) -> Diagnostic {
        self.note = Some(note.into());
        self
    }

    /// Render this diagnostic in a rustc-like format.
    ///
    /// `filename` is the path shown in the `-->` line. `source` is the full
    /// source text, used to extract the offending line for the caret display.
    pub fn render(&self, filename: &str, source: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!("error[{}]: {}\n", self.code, self.message));

        if let Some(span) = self.span {
            out.push_str(&format!(
                " --> {}:{}:{}\n",
                filename, span.line, span.column
            ));

            let line_text = source
                .lines()
                .nth(span.line.saturating_sub(1))
                .unwrap_or("");
            let gutter_width = span.line.to_string().len();
            let pad = " ".repeat(gutter_width);

            out.push_str(&format!("{pad} |\n"));
            out.push_str(&format!("{} | {}\n", span.line, line_text));

            // Caret underline. Column is 1-based in chars.
            let caret_pad = " ".repeat(span.column.saturating_sub(1));
            let caret_len = caret_len(source, span);
            let carets = "^".repeat(caret_len.max(1));
            out.push_str(&format!("{pad} | {caret_pad}{carets}\n"));

            if let Some(note) = &self.note {
                out.push_str(&format!("{pad} |\n"));
                out.push_str(&format!("{pad} = {note}\n"));
            }
        } else if let Some(note) = &self.note {
            out.push_str(&format!("  = {note}\n"));
        }

        out
    }
}

/// Number of carets to draw: the span length in chars, clamped to the end of
/// the span's line so the underline never spills onto the next line.
fn caret_len(source: &str, span: Span) -> usize {
    let start = span.start.min(source.len());
    let end = span.end.min(source.len());
    if end <= start {
        return 1;
    }
    let slice = &source[start..end];
    // Stop at the first newline so multi-line spans only underline line one.
    let first_line = slice.split('\n').next().unwrap_or(slice);
    first_line.chars().count().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_formatting_is_zero_padded() {
        assert_eq!(Code::WrongType.to_string(), "AION-008");
        assert_eq!(Code::Syntax.to_string(), "AION-001");
    }

    #[test]
    fn line_col_handles_multibyte() {
        let src = "aion 0.0.1-1;\nlanguage é {";
        // Offset at the 'l' of language on line 2.
        let offset = src.find("language").unwrap();
        let (line, col) = line_col(src, offset);
        assert_eq!(line, 2);
        assert_eq!(col, 1);
    }

    #[test]
    fn render_contains_code_and_location() {
        let src = "aion 0.0.1-1;\ncardinality = \"twenty\";";
        let span = Span::from_offsets(src, 28, 36);
        let d = Diagnostic::new(Code::WrongType, "incorrect value type")
            .with_span(span)
            .with_note("expected a nonnegative integer");
        let rendered = d.render("examples/invalid.aion", src);
        assert!(rendered.contains("error[AION-008]: incorrect value type"));
        assert!(rendered.contains("--> examples/invalid.aion:2:"));
        assert!(rendered.contains("^"));
        assert!(rendered.contains("= expected a nonnegative integer"));
    }
}
