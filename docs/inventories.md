# Inventories

The `phonology.inventory` block describes the concrete phoneme inventory of a
language — its consonants and vowels. It is **optional** and may appear at most
once.

```aion
phonology.inventory {
    consonants {
        cardinality = 18;
        include = { /m/, /n/, /ɳ/, /ʒ/ };
        exclude = { /q/ };
    }
    vowels {
        cardinality = 7;
        include = { /a/, /ə/, /i/ };
    }
}
```

Note the block keyword is spelled `phonology.inventory` — `phonology`, a dot,
then `inventory`.

## Inventory sections

The inventory contains two optional sub-blocks, each appearing at most once:

| Block | Keyword |
|-------|---------|
| Consonants | `consonants` |
| Vowels | `vowels` |

A repeated sub-block is reported as [`AION-004`](cli.md#diagnostic-codes).

## Properties

Both `consonants` and `vowels` accept the same three properties, each at most
once:

| Property | Type | Notes |
|----------|------|-------|
| `cardinality` | Integer (`u32`) | Target count of phonemes. |
| `include` | Phoneme set | Phonemes that must be present. |
| `exclude` | Phoneme set | Phonemes that must be absent. |

### `cardinality`

A nonnegative integer in the unsigned 32-bit range. A non-integer value is
reported as [`AION-008`](cli.md#diagnostic-codes); a value outside the `u32`
range is reported as [`AION-009`](cli.md#diagnostic-codes).

```aion
cardinality = 18;
```

### `include` and `exclude`

Sets of **phoneme literals**. A phoneme literal is delimited by forward slashes
and holds one or more characters that are not a slash or line break:

```aion
include = { /m/, /n/, /ɳ/, /ʒ/ };
exclude = { /q/ };
```

Phonemes may be any Unicode characters between the slashes, including IPA
symbols such as `/ɳ/`, `/ʒ/`, and `/ə/`. Phoneme-set members are compared by
exact literal content between the slashes, with no normalization; a duplicate
is reported as [`AION-010`](cli.md#diagnostic-codes).

!!! note "The validator does not interpret phonemes"
    The `aion` tool validates only the *structure* of phoneme literals. It does
    not verify that a literal is a real or well-formed IPA symbol, and it does
    not reconcile `cardinality` against the sizes of `include` or `exclude`.
    Those are design decisions left to the author.

## Common errors

| Situation | Code |
|-----------|------|
| `cardinality` set to a non-integer | [`AION-008`](cli.md#diagnostic-codes) |
| `cardinality` outside the `u32` range | [`AION-009`](cli.md#diagnostic-codes) |
| `include` / `exclude` not a phoneme set | [`AION-008`](cli.md#diagnostic-codes) |
| Duplicate phoneme in a set | [`AION-010`](cli.md#diagnostic-codes) |
| `consonants` or `vowels` repeated | [`AION-004`](cli.md#diagnostic-codes) |
