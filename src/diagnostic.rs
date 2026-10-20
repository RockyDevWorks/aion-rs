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

            let line_text = nth_line(source, span.line);
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
///
/// Byte offsets are snapped to UTF-8 character boundaries before slicing so
/// that a span whose bounds were clamped mid-codepoint (for example a
/// single-position syntax error reported just before a multi-byte character)
/// can never panic.
fn caret_len(source: &str, span: Span) -> usize {
    let start = floor_char_boundary(source, span.start.min(source.len()));
    let end = floor_char_boundary(source, span.end.min(source.len()));
    if end <= start {
        return 1;
    }
    let slice = &source[start..end];
    // Stop at the first line break so multi-line spans only underline line one.
    // Both `\n` and `\r` are treated as terminators so CR and CRLF endings
    // behave identically to LF.
    let first_line = slice.split(['\n', '\r']).next().unwrap_or(slice);
    first_line.chars().count().max(1)
}

/// Extract the 1-based `line`-th line of `source` for display.
///
/// Lines are delimited by `\n`, matching how [`line_col`] counts line numbers,
/// so the rendered line always corresponds to the reported line number. A
/// trailing `\r` (from a CRLF ending) is stripped so carriage returns are not
/// echoed into the diagnostic. Returns `""` when the line is out of range.
fn nth_line(source: &str, line: usize) -> &str {
    let raw = source.split('\n').nth(line.saturating_sub(1)).unwrap_or("");
    raw.strip_suffix('\r').unwrap_or(raw)
}

/// Round `index` down to the nearest UTF-8 character boundary (never past the
/// start of the string). Equivalent to the unstable `str::floor_char_boundary`.
fn floor_char_boundary(source: &str, index: usize) -> usize {
    let mut i = index.min(source.len());
    while i > 0 && !source.is_char_boundary(i) {
        i -= 1;
    }
    i
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
    fn caret_len_does_not_panic_on_mid_codepoint_span() {
        // `é` is two bytes (0xC3 0xA9). A span whose end falls between them must
        // not panic: `floor_char_boundary` snaps it back to a boundary.
        let src = "é";
        let span = Span {
            start: 0,
            end: 1, // mid-codepoint
            line: 1,
            column: 1,
        };
        // Should return 1 (the snapped-back slice is empty) without panicking.
        assert_eq!(caret_len(src, span), 1);
    }

    #[test]
    fn render_mid_codepoint_span_does_not_panic() {
        let src = "aion 0.0.1-1;\nlanguage café {";
        // Build a span that deliberately ends inside the `é` byte sequence.
        let start = src.find('é').unwrap();
        let span = Span {
            start,
            end: start + 1,
            line: 2,
            column: 15,
        };
        let d = Diagnostic::new(Code::Syntax, "syntax error").with_span(span);
        // Must not panic.
        let rendered = d.render("inline.aion", src);
        assert!(rendered.contains("error[AION-001]"));
    }

    #[test]
    fn nth_line_strips_trailing_cr_for_crlf() {
        let src = "aion 0.0.1-1;\r\nlanguage x {\r\n";
        assert_eq!(nth_line(src, 1), "aion 0.0.1-1;");
        assert_eq!(nth_line(src, 2), "language x {");
    }

    #[test]
    fn nth_line_out_of_range_is_empty() {
        assert_eq!(nth_line("one\ntwo", 99), "");
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

    #[test]
    fn all_codes_format_as_three_digit_aion() {
        let codes = [
            Code::Syntax,
            Code::UnsupportedVersion,
            Code::MissingRequired,
            Code::Duplicate,
            Code::WrongType,
            Code::IntegerOverflow,
            Code::DuplicateMember,
            Code::EmptyValue,
            Code::InvalidEscape,
            Code::Input,
        ];
        for c in codes {
            let s = c.to_string();
            assert!(s.starts_with("AION-"), "{s}");
            // Numeric part is zero-padded to at least three digits.
            let num = &s["AION-".len()..];
            assert!(num.len() >= 3, "{s}");
            assert!(num.chars().all(|ch| ch.is_ascii_digit()), "{s}");
            assert_eq!(c.number(), num.parse::<u16>().unwrap());
        }
    }

    #[test]
    fn render_without_span_but_with_note_uses_inline_note() {
        let d = Diagnostic::new(Code::MissingRequired, "something is missing")
            .with_note("add the required element");
        let rendered = d.render("f.aion", "aion 0.0.1-1;");
        assert!(rendered.contains("error[AION-003]: something is missing"));
        // No `-->` location line when there is no span.
        assert!(!rendered.contains("-->"));
        // The note is rendered inline with the `=` marker.
        assert!(rendered.contains("= add the required element"));
    }

    #[test]
    fn render_without_span_or_note_is_just_the_header() {
        let d = Diagnostic::new(Code::Syntax, "empty document");
        let rendered = d.render("f.aion", "");
        assert_eq!(rendered, "error[AION-001]: empty document\n");
    }

    #[test]
    fn render_span_without_note_has_no_note_line() {
        let src = "aion 0.0.1-1;";
        let span = Span::from_offsets(src, 0, 4);
        let d = Diagnostic::new(Code::Syntax, "bad").with_span(span);
        let rendered = d.render("f.aion", src);
        assert!(rendered.contains("-->"));
        assert!(!rendered.contains(" = "));
    }

    #[test]
    fn from_offsets_clamps_past_end() {
        let src = "abc";
        // Offsets beyond the end must clamp rather than panic.
        let span = Span::from_offsets(src, 100, 200);
        assert_eq!(span.line, 1);
    }

    #[test]
    fn caret_len_multiline_span_underlines_first_line_only() {
        let src = "line1\nline2";
        // Span covering both lines; caret length clamps to first line (5).
        let span = Span {
            start: 0,
            end: src.len(),
            line: 1,
            column: 1,
        };
        assert_eq!(caret_len(src, span), 5);
    }
}
