# AION

`aion` is a minimal, production-quality command-line **validator** for the
**AION v0.0.1-1** language-definition format. It reads an `.aion` file and
reports whether that file is *structurally* valid according to the AION
grammar.

AION is a small declarative format for describing the *intent* and *inventory*
of a constructed language. A definition names a language, carries mandatory
metadata, and may describe the phonological intent and a phoneme inventory.

## What the validator does

- Parses an `.aion` document against the authoritative Pest grammar.
- Checks structural rules: required blocks, allowed properties, value types,
  duplicate detection, integer ranges, and Unicode-escape validity.
- Emits rustc-style diagnostics with stable error codes, line/column spans,
  and explanatory notes.

## What it does not do

The validator checks definitions only. It does **not**:

- generate languages or phonemes,
- invoke any AI model,
- produce ELANG (or any other) output,
- interpret linguistic meaning, or
- modify the source file.

The source is never normalized or rewritten.

## The smallest valid definition

```aion
aion 0.0.1-1;

language aelith {
    metadata {
        description = "An ancient, melodic constructed language.";
    }
}
```

## Where to go next

- [Installation](installation.md) — build the `aion` binary from source.
- [Quick start](quick-start.md) — validate your first definition.
- [Syntax](syntax.md) — the shape of an AION document.
- [Supported types](types.md) — the value types a property may take.
- [Metadata](metadata.md), [Phonology intent](intent-phonology.md),
  [Inventories](inventories.md) — the three language sections.
- [CLI usage](cli.md) — commands, output, and exit codes.
- [Examples](examples.md) — minimal and complete definitions.

!!! note "Version scope"
    This documentation describes AION **v0.0.1-1**, the only version the
    validator currently accepts. The version string appears verbatim in every
    document: `aion 0.0.1-1;`.
