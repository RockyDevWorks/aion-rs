# Supported types

Every property in an AION document has the form `name = value;`. The value must
be one of the types below, and each property accepts a specific set of types.
A value whose type does not match the property is reported as
[`AION-008`](cli.md#diagnostic-codes) (incorrect value type).

## Value types

### String

A quoted or multiline string.

```aion
display_name = "Aelith";

description = """
    A multiline description that
    preserves its contents verbatim.
""";
```

Escape sequences in quoted strings: `\"`, `\\`, `\n`, `\r`, `\t`, and Unicode
escapes `\u{...}` (1–6 hex digits). An escape that does not resolve to a valid
Unicode scalar value is reported as [`AION-012`](cli.md#diagnostic-codes).

### Integer

A nonnegative decimal integer that fits in an unsigned 32-bit range (`u32`,
`0`–`4294967295`). No leading zeros, except `0` itself. A value outside the
`u32` range is reported as [`AION-009`](cli.md#diagnostic-codes).

```aion
cardinality = 18;
```

### String set

A brace-delimited, comma-separated set of strings.

```aion
style = { "melodic", "soft", "archaic" };
```

### Phoneme set

A set whose members are phoneme literals. A phoneme literal is delimited by
forward slashes and contains one or more characters that are not a slash or a
line break: `/m/`, `/ɳ/`, `/ə/`.

```aion
include = { /m/, /n/, /ɳ/, /ʒ/ };
```

### Reference set

A set whose members are typed references. A reference is `language("...")` or
`conlang("...")`, where the argument is a string.

```aion
references = {
    language("Tamil"),
    conlang("Quenya")
};
```

## Property-to-type map

| Property | Section | Accepted type(s) |
|----------|---------|------------------|
| `description` | metadata, intent phonology | String |
| `display_name` | metadata | String |
| `style` | intent phonology | String or string set |
| `character` | intent phonology | String or string set |
| `articulation` | intent phonology | String or string set |
| `rhythm` | intent phonology | String or string set |
| `avoid` | intent phonology | String or string set |
| `references` | intent phonology | Reference set |
| `cardinality` | inventory block | Integer |
| `include` | inventory block | Phoneme set |
| `exclude` | inventory block | Phoneme set |

!!! note "Set rules"
    Sets must be homogeneous — all members of a single kind. Trailing commas
    are not permitted. An empty set `{ }` is allowed by the grammar. See
    [Syntax](syntax.md#sets) for details.

## Duplicate member comparison

Duplicate members within a set are reported as
[`AION-010`](cli.md#diagnostic-codes). Members are compared as follows:

- **String sets** and **reference sets**: members are compared by their
  *decoded* value (escape sequences resolved). References additionally compare
  the kind (`language` versus `conlang`), so `language("X")` and `conlang("X")`
  are distinct members.
- **Phoneme sets**: literals are compared by their exact content between the
  delimiting slashes, with no normalization.

The source is never normalized or rewritten.
