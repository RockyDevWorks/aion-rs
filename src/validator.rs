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
                    // display_name must be a string, and its escapes must be
                    // valid Unicode scalar values (AION-012).
                    if let Some(value) = self.property_value(&inner) {
                        if self.check_value_type(&value, ValueKind::String) {
                            self.check_value_contents(&value, ValueKind::String);
                        }
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
    ///
    /// Comparison keys preserve the documented policy (see
    /// `docs/types.md#duplicate-member-comparison`): string sets compare by
    /// decoded value (`s:`), reference sets by kind plus decoded argument
    /// (`r:kind:`), and phoneme sets by exact literal content (`p:`). Membership
    /// is tracked in a [`HashSet`], so detection is linear in the number of
    /// members rather than quadratic.
    fn check_set_members(&mut self, set: &Pair<'s, Rule>, expected: ValueKind) {
        use std::collections::HashSet;
        let mut seen: HashSet<String> = HashSet::new();
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

            if !seen.insert(key) {
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

#[cfg(test)]
mod tests {
    use crate::check;
    use crate::diagnostic::Code;

    /// Wrap a language body in a minimal valid document.
    fn doc(body: &str) -> String {
        format!("aion 0.0.1-1;\nlanguage t {{\n    metadata {{\n        description = \"d\";\n    }}\n{body}\n}}\n")
    }

    #[test]
    fn large_unique_phoneme_set_validates() {
        // A large homogeneous set with no duplicates must validate. This
        // exercises the linear (HashSet) duplicate path on many members.
        let mut members = String::new();
        for i in 0..5000 {
            if i > 0 {
                members.push_str(", ");
            }
            members.push_str(&format!("/p{i}/"));
        }
        let body = format!(
            "    phonology.inventory {{\n        consonants {{\n            include = {{ {members} }};\n        }}\n    }}"
        );
        assert!(
            check(&doc(&body)).is_ok(),
            "large unique set should validate"
        );
    }

    #[test]
    fn large_set_with_one_duplicate_is_caught() {
        // 4000 unique phonemes plus one repeat; the single duplicate must be
        // reported even deep in a large set.
        let mut members = String::new();
        for i in 0..4000 {
            members.push_str(&format!("/p{i}/, "));
        }
        members.push_str("/p0/"); // duplicate of the first member
        let body = format!(
            "    phonology.inventory {{\n        consonants {{\n            include = {{ {members} }};\n        }}\n    }}"
        );
        let diags = check(&doc(&body)).expect_err("duplicate should be caught");
        assert!(diags.iter().any(|d| d.code == Code::DuplicateMember));
    }

    #[test]
    fn display_name_invalid_escape_is_reported() {
        // Regression: display_name escapes must be validated (AION-012).
        let body = ""; // metadata already carries display_name below
        let src = format!(
            "aion 0.0.1-1;\nlanguage t {{\n    metadata {{\n        description = \"d\";\n        display_name = \"x\\u{{110000}}\";\n    }}\n{body}}}\n"
        );
        let diags = check(&src).expect_err("invalid display_name escape");
        assert!(diags.iter().any(|d| d.code == Code::InvalidEscape));
    }

    #[test]
    fn display_name_valid_escape_accepted() {
        let src = "aion 0.0.1-1;\nlanguage t {\n    metadata {\n        description = \"d\";\n        display_name = \"Ael\\u{00ED}th\";\n    }\n}\n";
        assert!(check(src).is_ok(), "valid display_name escape should pass");
    }

    /// Grammar/validator agreement: for every property, a correctly typed
    /// value validates and every other value form is rejected as AION-008
    /// (incorrect value type). The grammar parses all value forms permissively;
    /// this pins the validator's type decision against that permissive surface.
    #[test]
    fn property_value_types_agree_with_grammar() {
        // Representative value forms the grammar accepts in value position.
        const STRING: &str = "\"x\"";
        const MULTILINE: &str = "\"\"\"x\"\"\"";
        const INT: &str = "7";
        const STRING_SET: &str = "{ \"a\", \"b\" }";
        const PHONEME_SET: &str = "{ /a/, /b/ }";
        const REFERENCE_SET: &str = "{ language(\"X\") }";
        const EMPTY_SET: &str = "{ }";

        fn intent(value_line: &str) -> String {
            format!("    intent phonology {{\n        {value_line}\n    }}")
        }
        fn inv(value_line: &str) -> String {
            format!("    phonology.inventory {{\n        consonants {{\n            {value_line}\n        }}\n    }}")
        }
        // Each wrapper builds a COMPLETE document (one metadata block with the
        // mandatory description) so there is no duplicate-block noise.
        fn whole(body: &str) -> String {
            format!("aion 0.0.1-1;\nlanguage t {{\n{body}\n}}\n")
        }
        fn with_meta_and(extra: &str) -> String {
            whole(&format!(
                "    metadata {{\n        description = \"d\";\n    }}\n{extra}"
            ))
        }

        // (wrapper, accepted forms, rejected forms) per property.
        type Wrap = fn(&str) -> String;
        let display_name: Wrap = |v| {
            whole(&format!(
                "    metadata {{\n        description = \"d\";\n        display_name = {v};\n    }}"
            ))
        };
        let style: Wrap = |v| with_meta_and(&intent(&format!("style = {v};")));
        let references: Wrap = |v| with_meta_and(&intent(&format!("references = {v};")));
        let cardinality: Wrap = |v| with_meta_and(&inv(&format!("cardinality = {v};")));
        let include: Wrap = |v| with_meta_and(&inv(&format!("include = {v};")));

        let cases: &[(&str, Wrap, &[&str], &[&str])] = &[
            // name, wrapper, accepted, rejected(AION-008)
            (
                "display_name",
                display_name,
                &[STRING, MULTILINE],
                &[INT, STRING_SET, PHONEME_SET, REFERENCE_SET],
            ),
            (
                "style",
                style,
                &[STRING, MULTILINE, STRING_SET, EMPTY_SET],
                &[INT, PHONEME_SET, REFERENCE_SET],
            ),
            (
                "references",
                references,
                &[REFERENCE_SET, EMPTY_SET],
                &[STRING, INT, STRING_SET, PHONEME_SET],
            ),
            (
                "cardinality",
                cardinality,
                &[INT],
                &[STRING, STRING_SET, PHONEME_SET, REFERENCE_SET, EMPTY_SET],
            ),
            (
                "include",
                include,
                &[PHONEME_SET, EMPTY_SET],
                &[STRING, INT, STRING_SET, REFERENCE_SET],
            ),
        ];

        for (name, wrap, accepted, rejected) in cases {
            for v in *accepted {
                let src = wrap(v);
                assert!(
                    check(&src).is_ok(),
                    "{name}: value `{v}` should be accepted\n{src}"
                );
            }
            for v in *rejected {
                let src = wrap(v);
                let diags = check(&src)
                    .err()
                    .unwrap_or_else(|| panic!("{name}: value `{v}` should be rejected\n{src}"));
                assert!(
                    diags.iter().any(|d| d.code == Code::WrongType),
                    "{name}: value `{v}` should be AION-008, got {:?}\n{src}",
                    diags.iter().map(|d| d.code).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn phoneme_duplicate_uses_exact_literal_no_normalization() {
        // Two phoneme literals that are byte-distinct are NOT duplicates, even
        // if they might normalize to the same NFC form. Policy: exact content.
        // U+00E9 (é) vs U+0065 U+0301 (e + combining acute) are distinct.
        let body = "    phonology.inventory {\n        consonants {\n            include = { /\u{00e9}/, /e\u{0301}/ };\n        }\n    }";
        assert!(
            check(&doc(body)).is_ok(),
            "byte-distinct phonemes must not be treated as duplicates"
        );
    }

    // ---- decode_quoted: direct unit tests ---------------------------------
    //
    // Several escape-handling arms in `decode_quoted` are defensive: the
    // grammar normally rejects the malformed forms before the validator runs,
    // so they are unreachable through `check()`. We exercise them directly on
    // the free function to lock in their behaviour and cover those branches.

    use super::{decode_quoted, Span};

    /// A throwaway span for decode tests; its values are irrelevant to decoding.
    fn span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }

    #[test]
    fn decode_quoted_simple_escapes_all_decode() {
        let decoded = decode_quoted(r#""a\"b\\c\nd\re\tf""#, span()).expect("valid escapes");
        assert_eq!(decoded, "a\"b\\c\nd\re\tf");
    }

    #[test]
    fn decode_quoted_plain_text_passthrough() {
        assert_eq!(decode_quoted("\"hello\"", span()).unwrap(), "hello");
    }

    #[test]
    fn decode_quoted_valid_unicode_escape() {
        assert_eq!(decode_quoted(r#""x\u{41}y""#, span()).unwrap(), "xAy");
    }

    #[test]
    fn decode_quoted_missing_brace_after_u() {
        let err = decode_quoted(r#""\uABCD""#, span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
        assert!(err.note.unwrap().contains("expected `{`"));
    }

    #[test]
    fn decode_quoted_empty_hex_is_rejected() {
        let err = decode_quoted(r#""\u{}""#, span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
        assert!(err.note.unwrap().contains("1-6 hexadecimal"));
    }

    #[test]
    fn decode_quoted_too_long_hex_is_rejected() {
        let err = decode_quoted(r#""\u{1234567}""#, span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
        assert!(err.note.unwrap().contains("1-6 hexadecimal"));
    }

    #[test]
    fn decode_quoted_non_hex_digit_is_rejected() {
        let err = decode_quoted(r#""\u{12G4}""#, span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
        assert!(err.note.unwrap().contains("1-6 hexadecimal"));
    }

    #[test]
    fn decode_quoted_surrogate_is_not_a_scalar() {
        let err = decode_quoted(r#""\u{D800}""#, span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
        assert!(err
            .note
            .unwrap()
            .contains("not a valid Unicode scalar value"));
    }

    #[test]
    fn decode_quoted_above_max_scalar_is_rejected() {
        let err = decode_quoted(r#""\u{110000}""#, span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
    }

    #[test]
    fn decode_quoted_unknown_escape_is_rejected() {
        let err = decode_quoted(r#""\x""#, span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
        assert!(err.note.unwrap().contains("unknown escape"));
    }

    #[test]
    fn decode_quoted_trailing_backslash_is_rejected() {
        // Raw string cannot end in a backslash before the closing quote using
        // r#""#, so build it explicitly: the literal `"\`.
        let err = decode_quoted("\"\\", span()).unwrap_err();
        assert_eq!(err.code, Code::InvalidEscape);
        assert!(err.note.unwrap().contains("trailing backslash"));
    }

    #[test]
    fn decode_quoted_max_scalar_boundary_is_valid() {
        // U+10FFFF is the highest valid scalar value.
        let decoded = decode_quoted(r#""\u{10FFFF}""#, span()).unwrap();
        assert_eq!(decoded, "\u{10FFFF}");
    }

    // ---- validator-level coverage for less-common paths -------------------

    #[test]
    fn conlang_and_language_references_are_distinct_not_duplicates() {
        // Same decoded argument but different kinds => NOT a duplicate.
        let body = "    intent phonology {\n        references = { language(\"X\"), conlang(\"X\") };\n    }";
        assert!(
            check(&doc(body)).is_ok(),
            "language(X) and conlang(X) are distinct members"
        );
    }

    #[test]
    fn duplicate_conlang_reference_is_caught() {
        let body = "    intent phonology {\n        references = { conlang(\"Q\"), conlang(\"Q\") };\n    }";
        let diags = check(&doc(body)).expect_err("duplicate conlang reference");
        assert!(diags.iter().any(|d| d.code == Code::DuplicateMember));
    }

    #[test]
    fn duplicate_string_set_member_is_caught() {
        let body = "    intent phonology {\n        style = { \"a\", \"a\" };\n    }";
        let diags = check(&doc(body)).expect_err("duplicate string member");
        assert!(diags.iter().any(|d| d.code == Code::DuplicateMember));
    }

    #[test]
    fn multiline_string_value_is_accepted_for_description() {
        let body = "    intent phonology {\n        description = \"\"\"multi\nline\"\"\";\n    }";
        assert!(check(&doc(body)).is_ok());
    }

    #[test]
    fn whitespace_only_multiline_description_is_empty_value() {
        let src = "aion 0.0.1-1;\nlanguage t {\n    metadata {\n        description = \"\"\"   \n   \"\"\";\n    }\n}\n";
        let diags = check(src).expect_err("whitespace-only description");
        assert!(diags.iter().any(|d| d.code == Code::EmptyValue));
    }

    #[test]
    fn reference_argument_escape_is_decoded_for_comparison() {
        // language("A") and language("\u{41}") decode to the same arg => dup.
        let body = "    intent phonology {\n        references = { language(\"A\"), language(\"\\u{41}\") };\n    }";
        let diags = check(&doc(body)).expect_err("escaped duplicate reference arg");
        assert!(diags.iter().any(|d| d.code == Code::DuplicateMember));
    }

    #[test]
    fn exclude_phoneme_set_is_validated() {
        let body = "    phonology.inventory {\n        vowels {\n            exclude = { /x/, /y/ };\n        }\n    }";
        assert!(check(&doc(body)).is_ok());
    }

    #[test]
    fn invalid_escape_in_phoneme_free_string_argument_of_reference_reported() {
        let body =
            "    intent phonology {\n        references = { language(\"bad\\u{D800}\") };\n    }";
        let diags = check(&doc(body)).expect_err("invalid escape in reference arg");
        assert!(diags.iter().any(|d| d.code == Code::InvalidEscape));
    }

    #[test]
    fn many_duplicate_members_do_not_exceed_diagnostic_cap() {
        // Build a set with far more than MAX_DIAGNOSTICS duplicates to exercise
        // the push cap. All members after the first are duplicates.
        let mut members = String::from("\"a\"");
        for _ in 0..(super::MAX_DIAGNOSTICS + 50) {
            members.push_str(", \"a\"");
        }
        let body = format!("    intent phonology {{\n        style = {{ {members} }};\n    }}");
        let diags = check(&doc(&body)).expect_err("many duplicates");
        assert!(diags.len() <= super::MAX_DIAGNOSTICS);
        assert!(diags.iter().all(|d| d.code == Code::DuplicateMember));
    }
}
