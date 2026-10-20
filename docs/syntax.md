# Syntax

This page describes the overall shape of an AION document. The authoritative
grammar is `grammar/aion.pest` in the repository, compiled directly into the
`aion` crate. This page is a human-readable companion to that grammar, not a
replacement for it.

## Document structure

An AION document consists of exactly two parts, in this order:

1. A single version declaration: `aion 0.0.1-1;`
2. A single language declaration: `language <identifier> { ... }`

```aion
aion 0.0.1-1;

language aelith {
    // sections...
}
```

Content after the closing brace of the language declaration is a syntax error.

## Version declaration

The version declaration is literally:

```aion
aion 0.0.1-1;
```

Only `0.0.1-1` is accepted. Any other version is reported as
[`AION-002`](cli.md#diagnostic-codes) (unsupported version).

## Language declaration

```aion
language <identifier> {
    // language sections
}
```

- `<identifier>` starts with an ASCII letter or underscore and continues with
  ASCII letters, digits, or underscores. Reserved keywords (such as `metadata`
  or `phonology`) cannot be used as the identifier.
- The body contains zero or more **language sections**.

## Language sections

Three sections are available:

| Section | Keyword | Required | Max occurrences |
|---------|---------|----------|-----------------|
| [Metadata](metadata.md) | `metadata` | Yes | 1 |
| [Phonology intent](intent-phonology.md) | `intent phonology` | No | 1 |
| [Inventory](inventories.md) | `phonology.inventory` | No | 1 |

Sections may appear **in any order**. Each section may appear **at most once**,
and within a block each property may appear **at most once**. A repeated block
or property is reported as [`AION-004`](cli.md#diagnostic-codes).

## Properties

Every property has the form:

```aion
name = value;
```

The trailing semicolon is required. The accepted value types are described on
the [Supported types](types.md) page.

## Comments

Both comment styles are supported and may appear anywhere whitespace is
allowed:

```aion
// line comment

/* block
   comment */
```

## Strings

Two string forms exist:

- **Quoted strings** use double quotes and support escape sequences:
  `\"`, `\\`, `\n`, `\r`, `\t`, and Unicode escapes `\u{...}` (1–6 hex digits).
  An escape that is not a valid Unicode scalar value is reported as
  [`AION-012`](cli.md#diagnostic-codes).

    ```aion
    description = "A language with a \u{2018}soft\u{2019} feel.";
    ```

- **Multiline strings** are delimited by triple quotes and preserve their
  contents verbatim:

    ```aion
    description = """
        An ancient, melodic constructed language spoken
        by the mountain clans of the northern reaches.
    """;
    ```

## Sets

A set is a brace-delimited, comma-separated list of members of a single kind:

```aion
style = { "melodic", "soft", "archaic" };
include = { /m/, /n/, /ɳ/ };
references = { language("Tamil"), conlang("Quenya") };
```

- **Trailing commas are not allowed** and are reported as a syntax error
  ([`AION-001`](cli.md#diagnostic-codes)).
- An empty set `{ }` is permitted by the grammar.
- Members must all be the same kind (all strings, all phonemes, or all
  references). Mixed or disallowed member kinds are rejected by the validator.
- Duplicate members are reported as [`AION-010`](cli.md#diagnostic-codes). See
  [Supported types](types.md#duplicate-member-comparison) for the comparison
  policy.

## Integers

Integers are nonnegative decimal literals with no leading zeros (other than
`0` itself). They must fit in an unsigned 32-bit range; a value outside that
range is reported as [`AION-009`](cli.md#diagnostic-codes).
