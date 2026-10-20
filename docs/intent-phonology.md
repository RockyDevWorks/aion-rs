# Phonology intent

The `intent phonology` block records the *design intent* for a language's
phonology — the qualitative goals a language author has in mind. It is
**optional** and may appear at most once.

```aion
intent phonology {
    description = "Soft, flowing, vowel-forward phonology.";
    style = { "melodic", "soft", "archaic" };
    character = "liquid";
    articulation = { "front", "open" };
    rhythm = "trochaic";
    avoid = { "harsh plosives", "consonant clusters" };
    references = {
        language("Tamil"),
        conlang("Quenya")
    };
}
```

## Properties

| Property | Type | Notes |
|----------|------|-------|
| `description` | String | Free-form prose describing the intent. |
| `style` | String or string set | Free-form descriptive. |
| `character` | String or string set | Free-form descriptive. |
| `articulation` | String or string set | Free-form descriptive. |
| `rhythm` | String or string set | Free-form descriptive. |
| `avoid` | String or string set | Qualities to steer away from. |
| `references` | Reference set | Typed `language(...)` / `conlang(...)` references. |

Each property may appear at most once; a repeat is reported as
[`AION-004`](cli.md#diagnostic-codes).

## Free-form descriptive properties

!!! important "`style`, `character`, `rhythm`, and `articulation` are free-form"
    These four properties are **free-form descriptive** values. The validator
    checks only their *shape* — that each is a string or a set of strings. It
    does **not** check the words you use against any fixed list, and it does
    **not** attach meaning to them. Any string (or set of strings) that parses
    is structurally valid.

Because these properties are free-form, you are free to use whatever
vocabulary best captures your intent. Each accepts either a single string or a
set of strings:

```aion
character = "liquid";                 // single string
style = { "melodic", "soft" };        // set of strings
```

### Recommended vocabulary (nonbinding)

The terms below are offered **only as suggestions** to help you get started.
They are **nonbinding**: they are not validated, not required, not exhaustive,
and carry no special meaning to the `aion` tool. You may use any of them, none
of them, or entirely different words. Ignoring these suggestions does not make
a definition any less valid.

!!! note "Nonbinding suggestions, not a controlled vocabulary"
    None of the words below are keywords or reserved values. The validator
    neither recognizes nor rejects them specifically — they are treated like
    any other string. Treat this list as a starting palette, not a rulebook.

- **`style`** — overall aesthetic, for example: `melodic`, `soft`, `harsh`,
  `archaic`, `guttural`, `flowing`.
- **`character`** — qualitative texture, for example: `liquid`, `sibilant`,
  `nasal`, `percussive`, `breathy`.
- **`rhythm`** — prosodic feel, for example: `trochaic`, `iambic`,
  `syllable-timed`, `staccato`, `lilting`.
- **`articulation`** — articulatory tendencies, for example: `front`, `back`,
  `open`, `close`, `rounded`, `tense`.

Again, these are illustrative starting points only. Substitute your own terms
freely.

### `avoid`

`avoid` records qualities to steer away from. Like the descriptive properties
above, it is free-form and accepts a string or a set of strings:

```aion
avoid = { "harsh plosives", "consonant clusters" };
```

## `description`

A free-form string summarizing the phonological intent. Unlike the metadata
`description`, the intent `description` is optional.

```aion
description = "Soft, flowing, vowel-forward phonology.";
```

## `references`

`references` is a set of **typed references** to existing languages that
inspire the design. Each member is `language("...")` for a natural language or
`conlang("...")` for a constructed language:

```aion
references = {
    language("Tamil"),
    conlang("Quenya")
};
```

A reference set compares members by decoded value *and* kind, so
`language("Quenya")` and `conlang("Quenya")` are distinct. A repeated member is
reported as [`AION-010`](cli.md#diagnostic-codes).

## Relationship to the inventory

The `intent phonology` block captures *goals*; the
[`phonology.inventory`](inventories.md) block captures *concrete* consonants
and vowels. They are independent, optional sections — you may provide either,
both, or neither.
