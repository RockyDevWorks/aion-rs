//! Structural validator for AION v0.0.1-1.
//!
//! The validator walks the Pest parse tree produced by [`crate::parser`] and
//! enforces the structural rules of AION v0.0.1-1:
//!
//! * exactly one supported version (`0.0.1-1`) and one language declaration,
//! * a mandatory metadata block with a mandatory, non-whitespace description,
//! * optional `intent phonology` and `phonology.inventory` blocks, each at most
//!   once, with each property at most once,
//! * value-type enforcement (string, string-set, reference-set, integer,
//!   phoneme-set), u32 range for integers,
//! * duplicate set members rejected, trailing commas rejected (grammar),
//! * unicode escapes validated as scalar values.
//!
//! The grammar parses property values permissively through a single `value`
//! rule; this module decides whether the value's actual type matches the
//! property and reports [`Code::WrongType`] (AION-008) on a mismatch.
//!
//! It never prints and never panics; it returns a list of [`Diagnostic`]s.
//! An empty list means the definition is structurally valid.

use crate::diagnostic::{Code, Diagnostic, Span};
use crate::parser::GrammarRule as Rule;
use pest::iterators::Pair;

/// The only supported AION version.
pub const SUPPORTED_VERSION: &str = "0.0.1-1";

/// Maximum number of diagnostics collected before the validator stops adding
/// more. Documented limit to avoid unbounded output on pathological input.
pub const MAX_DIAGNOSTICS: usize = 100;

/// The expected shape of a property value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    /// A single string.
    String,
    /// A single string or a set of strings.
    StringOrStringSet,
    /// A set of typed references.
    ReferenceSet,
    /// An unsigned 32-bit integer.
    U32,
    /// A set of phoneme literals.
    PhonemeSet,
}

impl ValueKind {
    fn describe(self) -> &'static str {
        match self {
            ValueKind::String => "a string",
            ValueKind::StringOrStringSet => "a string or a set of strings",
            ValueKind::ReferenceSet => "a set of references",
            ValueKind::U32 => "a nonnegative integer",
            ValueKind::PhonemeSet => "a set of phoneme literals",
        }
    }
}

/// Validate an already-parsed AION `document` pair.
///
/// Returns an empty vector when the definition is structurally valid, otherwise
/// a list of diagnostics (capped at [`MAX_DIAGNOSTICS`]).
pub fn validate(source: &str, document: Pair<'_, Rule>) -> Vec<Diagnostic> {
    let mut v = Validator {
        source,
        diags: Vec::new(),
    };
    v.check_document(document);
    v.diags
}

struct Validator<'s> {
    source: &'s str,
    diags: Vec<Diagnostic>,
}

/// Build a [`Span`] from a pair's source span.
fn span_of(source: &str, pair: &Pair<'_, Rule>) -> Span {
    let s = pair.as_span();
    Span::from_offsets(source, s.start(), s.end())
}

impl<'s> Validator<'s> {
    fn push(&mut self, d: Diagnostic) {
        if self.diags.len() < MAX_DIAGNOSTICS {
            self.diags.push(d);
        }
    }

    fn at_capacity(&self) -> bool {
        self.diags.len() >= MAX_DIAGNOSTICS
    }

    // ---- document ----------------------------------------------------------

    fn check_document(&mut self, document: Pair<'s, Rule>) {
        for pair in document.into_inner() {
            match pair.as_rule() {
                Rule::version_declaration => self.check_version(pair),
                Rule::language_declaration => self.check_language(pair),
                _ => {}
            }
        }
    }

    fn check_version(&mut self, decl: Pair<'s, Rule>) {
        if let Some(version) = decl.into_inner().find(|p| p.as_rule() == Rule::version) {
            let text = version.as_str();
            if text != SUPPORTED_VERSION {
                self.push(
                    Diagnostic::new(
                        Code::UnsupportedVersion,
                        format!("unsupported AION version `{text}`"),
                    )
                    .with_span(span_of(self.source, &version))
                    .with_note(format!("only version `{SUPPORTED_VERSION}` is supported")),
                );
            }
        }
    }

    // ---- language ----------------------------------------------------------

    fn check_language(&mut self, lang: Pair<'s, Rule>) {
        let mut metadata_spans: Vec<Span> = Vec::new();
        let mut intent_spans: Vec<Span> = Vec::new();
        let mut inventory_spans: Vec<Span> = Vec::new();

        for section in lang.into_inner() {
            if section.as_rule() != Rule::language_section {
                continue;
            }
            let Some(block) = section.into_inner().next() else {
                continue;
            };
            match block.as_rule() {
                Rule::metadata_block => {
                    metadata_spans.push(span_of(self.source, &block));
                    self.check_metadata(block);
                }
                Rule::intent_phonology_block => {
                    intent_spans.push(span_of(self.source, &block));
                    self.check_intent(block);
                }
                Rule::phonology_inventory_block => {
                    inventory_spans.push(span_of(self.source, &block));
                    self.check_inventory(block);
                }
                _ => {}
            }
        }

        match metadata_spans.len() {
            0 => self.push(
                Diagnostic::new(
                    Code::MissingRequired,
                    "language is missing a required `metadata` block",
                )
                .with_note("every language must declare a `metadata` block"),
            ),
            1 => {}
            _ => self.duplicate_blocks("metadata", &metadata_spans),
        }

        if intent_spans.len() > 1 {
            self.duplicate_blocks("intent phonology", &intent_spans);
        }
        if inventory_spans.len() > 1 {
            self.duplicate_blocks("phonology.inventory", &inventory_spans);
        }
    }

    fn duplicate_blocks(&mut self, name: &str, spans: &[Span]) {
        for span in spans.iter().skip(1) {
            if self.at_capacity() {
                break;
            }
            self.push(
                Diagnostic::new(Code::Duplicate, format!("duplicate `{name}` block"))
                    .with_span(*span)
                    .with_note(format!("`{name}` may appear at most once per language")),
            );
        }
    }

    fn duplicate_property(&mut self, name: &str, spans: &[Span]) {
        for span in spans.iter().skip(1) {
            if self.at_capacity() {
                break;
            }
            self.push(
                Diagnostic::new(Code::Duplicate, format!("duplicate `{name}` property"))
                    .with_span(*span)
                    .with_note(format!("`{name}` may appear at most once in its block")),
            );
        }
    }

    // ---- metadata ----------------------------------------------------------

    fn check_metadata(&mut self, block: Pair<'s, Rule>) {
        let mut description_props: Vec<Pair<'s, Rule>> = Vec::new();
        let mut display_name_spans: Vec<Span> = Vec::new();

        for prop in block.into_inner() {
            if prop.as_rule() != Rule::metadata_property {
                continue;
            }
            let Some(inner) = prop.into_inner().next() else {
                continue;
            };
            match inner.as_rule() {
                Rule::metadata_description => description_props.push(inner),
                Rule::metadata_display_name => {
                    display_name_spans.push(span_of(self.source, &inner));
                    // display_name must be a string.
                    if let Some(value) = self.property_value(&inner) {
                        self.check_value_type(&value, ValueKind::String);
                    }
                }
                _ => {}
            }
        }

        match description_props.len() {
            0 => self.push(
                Diagnostic::new(
                    Code::MissingRequired,
                    "metadata is missing the required `description` property",
                )
                .with_note("`description` must be present and contain non-whitespace content"),
            ),
            1 => {
                let prop = description_props.into_iter().next().unwrap();
                self.check_description_content(prop);
            }
            _ => {
                let spans: Vec<Span> = description_props
                    .iter()
                    .map(|p| span_of(self.source, p))
                    .collect();
                self.duplicate_property("description", &spans);
                if let Some(prop) = description_props.into_iter().next() {
                    self.check_description_content(prop);
                }
            }
        }

        if display_name_spans.len() > 1 {
            self.duplicate_property("display_name", &display_name_spans);
        }
    }

    /// The `value` child of a property rule, if present.
    fn property_value(&self, prop: &Pair<'s, Rule>) -> Option<Pair<'s, Rule>> {
        prop.clone()
            .into_inner()
            .find(|p| p.as_rule() == Rule::value)
    }

    /// Enforce that `description` is a string with non-whitespace content.
    fn check_description_content(&mut self, prop: Pair<'s, Rule>) {
        let Some(value) = self.property_value(&prop) else {
            return;
        };
        if !self.check_value_type(&value, ValueKind::String) {
            return;
        }
        // value -> string
        let Some(string_pair) = value.into_inner().find(|p| p.as_rule() == Rule::string) else {
            return;
        };
        let span = span_of(self.source, &string_pair);
        match self.decode_string(&string_pair) {
            Ok(decoded) => {
                if decoded.trim().is_empty() {
                    self.push(
                        Diagnostic::new(
                            Code::EmptyValue,
                            "`description` must contain non-whitespace content",
                        )
                        .with_span(span)
                        .with_note("provide a descriptive, non-empty string"),
                    );
                }
            }
            Err(d) => self.push(d),
        }
    }

    // ---- intent phonology --------------------------------------------------

    fn check_intent(&mut self, block: Pair<'s, Rule>) {
        use std::collections::HashMap;
        let mut seen: HashMap<&'static str, Vec<Span>> = HashMap::new();

        for prop in block.into_inner() {
            if prop.as_rule() != Rule::intent_property {
                continue;
            }
            let Some(inner) = prop.into_inner().next() else {
                continue;
            };
            let span = span_of(self.source, &inner);
            let (name, expected) = match inner.as_rule() {
                Rule::intent_description => ("description", ValueKind::String),
                Rule::intent_style => ("style", ValueKind::StringOrStringSet),
                Rule::intent_references => ("references", ValueKind::ReferenceSet),
                Rule::intent_character => ("character", ValueKind::StringOrStringSet),
                Rule::intent_articulation => ("articulation", ValueKind::StringOrStringSet),
                Rule::intent_rhythm => ("rhythm", ValueKind::StringOrStringSet),
                Rule::intent_avoid => ("avoid", ValueKind::StringOrStringSet),
                _ => continue,
            };
            seen.entry(name).or_default().push(span);

            if let Some(value) = self.property_value(&inner) {
                if self.check_value_type(&value, expected) {
                    self.check_value_contents(&value, expected);
                }
            }
        }

        self.report_duplicate_map(&seen);
    }

    // ---- phonology inventory -----------------------------------------------

    fn check_inventory(&mut self, block: Pair<'s, Rule>) {
        let mut consonant_spans: Vec<Span> = Vec::new();
        let mut vowel_spans: Vec<Span> = Vec::new();

        for section in block.into_inner() {
            if section.as_rule() != Rule::inventory_section {
                continue;
            }
            let Some(inner) = section.into_inner().next() else {
                continue;
            };
            match inner.as_rule() {
                Rule::consonant_block => {
                    consonant_spans.push(span_of(self.source, &inner));
                    self.check_inventory_properties(inner);
                }
                Rule::vowel_block => {
                    vowel_spans.push(span_of(self.source, &inner));
                    self.check_inventory_properties(inner);
                }
                _ => {}
            }
        }

        if consonant_spans.len() > 1 {
            self.duplicate_blocks("consonants", &consonant_spans);
        }
        if vowel_spans.len() > 1 {
            self.duplicate_blocks("vowels", &vowel_spans);
        }
    }

    fn check_inventory_properties(&mut self, block: Pair<'s, Rule>) {
        use std::collections::HashMap;
        let mut seen: HashMap<&'static str, Vec<Span>> = HashMap::new();

        for prop in block.into_inner() {
            if prop.as_rule() != Rule::inventory_property {
                continue;
            }
            let Some(inner) = prop.into_inner().next() else {
                continue;
            };
            let span = span_of(self.source, &inner);
            let (name, expected) = match inner.as_rule() {
                Rule::cardinality_property => ("cardinality", ValueKind::U32),
                Rule::include_property => ("include", ValueKind::PhonemeSet),
                Rule::exclude_property => ("exclude", ValueKind::PhonemeSet),
                _ => continue,
            };
            seen.entry(name).or_default().push(span);

            if let Some(value) = self.property_value(&inner) {
                if self.check_value_type(&value, expected) {
                    self.check_value_contents(&value, expected);
                }
            }
        }

        self.report_duplicate_map(&seen);
    }

    fn report_duplicate_map(&mut self, seen: &std::collections::HashMap<&'static str, Vec<Span>>) {
        let mut names: Vec<&&'static str> = seen.keys().collect();
        names.sort();
        for name in names {
            if let Some(spans) = seen.get(*name) {
                if spans.len() > 1 {
                    self.duplicate_property(name, spans);
                }
            }
        }
    }

    // ---- value typing ------------------------------------------------------

    /// Check that a parsed `value` matches `expected`. Returns `true` when the
    /// type matches (so the caller may continue inspecting contents), `false`
    /// after pushing a [`Code::WrongType`] diagnostic.
    fn check_value_type(&mut self, value: &Pair<'s, Rule>, expected: ValueKind) -> bool {
        let Some(actual) = value.clone().into_inner().next() else {
            return false;
        };
        let span = span_of(self.source, value);

        let ok = match expected {
            ValueKind::String => actual.as_rule() == Rule::string,
            ValueKind::U32 => actual.as_rule() == Rule::integer,
            ValueKind::StringOrStringSet => {
                matches!(actual.as_rule(), Rule::string)
                    || (actual.as_rule() == Rule::set && self.set_is_of(&actual, SetElem::String))
            }
            ValueKind::ReferenceSet => {
                actual.as_rule() == Rule::set && self.set_is_of(&actual, SetElem::Reference)
            }
            ValueKind::PhonemeSet => {
                actual.as_rule() == Rule::set && self.set_is_of(&actual, SetElem::Phoneme)
            }
        };

        if !ok {
            self.push(
                Diagnostic::new(Code::WrongType, "incorrect value type")
                    .with_span(span)
                    .with_note(format!("expected {}", expected.describe())),
            );
        }
        ok
    }

    /// Whether every member of a `set` is of the given element kind.
    fn set_is_of(&self, set: &Pair<'s, Rule>, elem: SetElem) -> bool {
        for member in set.clone().into_inner() {
            if member.as_rule() != Rule::set_member {
                continue;
            }
            let Some(inner) = member.into_inner().next() else {
                continue;
            };
            let matches = match elem {
                SetElem::String => inner.as_rule() == Rule::string,
                SetElem::Reference => inner.as_rule() == Rule::reference,
                SetElem::Phoneme => inner.as_rule() == Rule::phoneme,
            };
            if !matches {
                return false;
            }
        }
        true
    }

    /// Inspect the contents of a type-checked value: decode strings/escapes,
    /// verify u32 range, and reject duplicate set members.
    fn check_value_contents(&mut self, value: &Pair<'s, Rule>, expected: ValueKind) {
        let Some(actual) = value.clone().into_inner().next() else {
            return;
        };
        match (expected, actual.as_rule()) {
            (ValueKind::String, Rule::string) | (ValueKind::StringOrStringSet, Rule::string) => {
                if let Err(d) = self.decode_string(&actual) {
                    self.push(d);
                }
            }
            (ValueKind::U32, Rule::integer) => self.check_u32(&actual),
            (_, Rule::set) => self.check_set_members(&actual, expected),
            _ => {}
        }
    }

    fn check_u32(&mut self, int: &Pair<'s, Rule>) {
        let span = span_of(self.source, int);
        let text = int.as_str();
        if text.parse::<u32>().is_err() {
            self.push(
                Diagnostic::new(
                    Code::IntegerOverflow,
                    format!("integer `{text}` is out of range"),
                )
                .with_span(span)
                .with_note(format!(
                    "value must be an unsigned 32-bit integer (0..={})",
                    u32::MAX
                )),
            );
        }
    }

    /// Reject duplicate members in a set, decoding strings/references and
    /// comparing phonemes by exact literal content.
    fn check_set_members(&mut self, set: &Pair<'s, Rule>, expected: ValueKind) {
        let mut seen: Vec<(String, Span)> = Vec::new();
        for member in set.clone().into_inner() {
            if member.as_rule() != Rule::set_member {
                continue;
            }
            let member_span = span_of(self.source, &member);
            let Some(inner) = member.into_inner().next() else {
                continue;
            };
            let key = match inner.as_rule() {
                Rule::string => match self.decode_string(&inner) {
                    Ok(decoded) => format!("s:{decoded}"),
                    Err(d) => {
                        self.push(d);
                        continue;
                    }
                },
                Rule::reference => match self.reference_key(&inner) {
                    Ok(k) => k,
                    Err(d) => {
                        self.push(d);
                        continue;
                    }
                },
                // Phoneme duplicate policy: exact literal content, no
                // normalization.
                Rule::phoneme => format!("p:{}", inner.as_str()),
                _ => continue,
            };

            if seen.iter().any(|(k, _)| *k == key) {
                let label = match expected {
                    ValueKind::ReferenceSet => "duplicate reference",
                    ValueKind::PhonemeSet => "duplicate phoneme literal",
                    _ => "duplicate set member",
                };
                self.push(
                    Diagnostic::new(Code::DuplicateMember, label.to_string())
                        .with_span(member_span)
                        .with_note("set members must be unique by their decoded/literal value"),
                );
            } else {
                seen.push((key, member_span));
            }
        }
    }

    /// Build a comparison key for a reference (kind + decoded string argument).
    fn reference_key(&self, reference: &Pair<'s, Rule>) -> Result<String, Diagnostic> {
        let mut kind = String::new();
        let mut arg: Option<Pair<'s, Rule>> = None;
        for part in reference.clone().into_inner() {
            match part.as_rule() {
                Rule::reference_kind => kind = part.as_str().to_string(),
                Rule::string => arg = Some(part),
                _ => {}
            }
        }
        match arg {
            Some(arg) => {
                let decoded = self.decode_string(&arg)?;
                Ok(format!("r:{kind}:{decoded}"))
            }
            None => Ok(format!("r:{kind}:")),
        }
    }

    // ---- string decoding ---------------------------------------------------

    /// Decode a `string` pair (quoted or multiline) into its textual value,
    /// validating escape sequences. Returns a diagnostic on invalid escapes.
    fn decode_string(&self, string_pair: &Pair<'s, Rule>) -> Result<String, Diagnostic> {
        let span = span_of(self.source, string_pair);
        let inner = string_pair
            .clone()
            .into_inner()
            .next()
            .unwrap_or_else(|| string_pair.clone());
        let raw = inner.as_str();

        match inner.as_rule() {
            Rule::multiline_string => {
                let body = raw
                    .strip_prefix("\"\"\"")
                    .and_then(|s| s.strip_suffix("\"\"\""))
                    .unwrap_or(raw);
                Ok(body.to_string())
            }
            _ => decode_quoted(raw, span),
        }
    }
}

/// Which concrete element a set is expected to contain.
#[derive(Clone, Copy)]
enum SetElem {
    String,
    Reference,
    Phoneme,
}

/// Decode a quoted string literal (including surrounding quotes) into its value,
/// validating all escape sequences. `span` is used for diagnostics.
fn decode_quoted(raw: &str, span: Span) -> Result<String, Diagnostic> {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);

    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                if chars.next() != Some('{') {
                    return Err(invalid_escape(span, "expected `{` after `\\u`"));
                }
                let mut hex = String::new();
                for ch in chars.by_ref() {
                    if ch == '}' {
                        break;
                    }
                    hex.push(ch);
                }
                if hex.is_empty() || hex.len() > 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(invalid_escape(
                        span,
                        "unicode escape must be 1-6 hexadecimal digits",
                    ));
                }
                let value = u32::from_str_radix(&hex, 16)
                    .map_err(|_| invalid_escape(span, "invalid unicode escape value"))?;
                match char::from_u32(value) {
                    Some(decoded) => out.push(decoded),
                    None => {
                        return Err(invalid_escape(
                            span,
                            format!(
                                "`\\u{{{hex}}}` (U+{value:X}) is not a valid Unicode scalar value"
                            ),
                        ))
                    }
                }
            }
            Some(other) => {
                return Err(invalid_escape(
                    span,
                    format!("unknown escape sequence `\\{other}`"),
                ))
            }
            None => return Err(invalid_escape(span, "trailing backslash in string")),
        }
    }

    Ok(out)
}

fn invalid_escape(span: Span, note: impl Into<String>) -> Diagnostic {
    Diagnostic::new(Code::InvalidEscape, "invalid escape sequence in string")
        .with_span(span)
        .with_note(note)
}
