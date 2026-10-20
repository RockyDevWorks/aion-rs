# Quick start

This page takes you from a clean checkout to a validated definition in a few
minutes. It assumes you have already [installed](installation.md) the Rust
toolchain.

## 1. Write a definition

Create a file named `aelith.aion`:

```aion
aion 0.0.1-1;

language aelith {
    metadata {
        description = "An ancient, melodic constructed language.";
    }
}
```

Every document starts with the version declaration `aion 0.0.1-1;`, followed by
exactly one `language <identifier> { ... }` declaration. The `metadata` block
with a non-empty `description` is the only mandatory section.

## 2. Validate it

```sh
cargo run -- check aelith.aion
```

Expected output:

```
Valid AION definition
```

The process exits with code `0`.

## 3. Introduce an error

Change `cardinality` to a string to see a diagnostic. Add a phonology
inventory with a type mismatch:

```aion
aion 0.0.1-1;

language aelith {
    metadata {
        description = "An ancient, melodic constructed language.";
    }

    phonology.inventory {
        consonants {
            cardinality = "twenty";
        }
    }
}
```

Validating it prints a rustc-style diagnostic and exits with code `1`:

```
error[AION-008]: incorrect value type
 --> aelith.aion:10:27
   |
10 |             cardinality = "twenty";
   |                           ^^^^^^^^
   |
   = expected a nonnegative integer

Invalid AION definition
```

## 4. Grow the definition

Add the optional sections as you need them:

- [Metadata](metadata.md) — `description` (required) and `display_name`.
- [Phonology intent](intent-phonology.md) — descriptive design goals such as
  `style`, `character`, `rhythm`, and `articulation`.
- [Inventories](inventories.md) — `consonants` and `vowels` with
  `cardinality`, `include`, and `exclude`.

A definition that exercises every supported section is shown on the
[Examples](examples.md) page.

## Next steps

- Read the [Syntax](syntax.md) rules for the document shape.
- Review the [Supported types](types.md) each property accepts.
- See [CLI usage](cli.md) for all commands and exit codes.
