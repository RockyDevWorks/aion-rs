# Examples

The repository ships runnable example files under `examples/`. This page
reproduces them and explains what each demonstrates. Validate any of them with:

```sh
cargo run -- check examples/<file>.aion
```

## Minimal

`examples/minimal.aion` — the smallest valid document: a version declaration, a
language, and a `metadata` block with a `description`.

```aion
aion 0.0.1-1;

language aelith {
    metadata {
        description = "An ancient, melodic constructed language.";
    }
}
```

## Complete

`examples/complete.aion` — exercises every supported section: metadata (with a
multiline description and a display name), phonology intent (including the
free-form descriptive properties and typed references), and a phonology
inventory for both consonants and vowels.

```aion
// A complete AION definition exercising every supported section.
aion 0.0.1-1;

language aelith {
    /* Metadata is mandatory and must carry a non-empty description. */
    metadata {
        description = """
            An ancient, melodic constructed language spoken
            by the mountain clans of the northern reaches.
        """;
        display_name = "Aelith";
    }

    intent phonology {
        description = "Soft, flowing, vowel-forward phonology.";
        style = { "melodic", "soft", "archaic" };
        references = {
            language("Tamil"),
            conlang("Quenya")
        };
        character = "liquid";
        articulation = { "front", "open" };
        rhythm = "trochaic";
        avoid = { "harsh plosives", "consonant clusters" };
    }

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
}
```

Notice that the sections can appear in any order, each at most once. In this
example `character` and `articulation` appear after `references`, which is
fine.

## Invalid

`examples/invalid.aion` — a definition that fails validation, used to
illustrate diagnostics. Validating it produces an [`AION-008`](cli.md#diagnostic-codes)
diagnostic because `cardinality` is given a string instead of an integer:

```
error[AION-008]: incorrect value type
 --> examples/invalid.aion:10:27
   |
10 |             cardinality = "twenty";
   |                           ^^^^^^^^
   |
   = expected a nonnegative integer

Invalid AION definition
```

## Reordered sections

Sections may appear in any order. This is just as valid as the canonical
ordering:

```aion
aion 0.0.1-1;

language aelith {
    phonology.inventory {
        vowels {
            cardinality = 5;
        }
    }

    metadata {
        description = "Sections may appear in any order.";
    }
}
```

## Empty sets

An empty set is accepted by the grammar:

```aion
aion 0.0.1-1;

language aelith {
    metadata {
        description = "Empty sets are permitted.";
    }

    intent phonology {
        style = { };
    }
}
```

More fixtures covering valid and invalid cases live under
`tests/fixtures/valid/` and `tests/fixtures/invalid/` in the repository.
