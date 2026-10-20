# Metadata

The `metadata` block describes the language itself. It is the only **mandatory**
language section: every valid document contains exactly one `metadata` block.

```aion
language aelith {
    metadata {
        description = "An ancient, melodic constructed language.";
        display_name = "Aelith";
    }
}
```

## Properties

| Property | Type | Required | Notes |
|----------|------|----------|-------|
| `description` | String | Yes | Must contain non-whitespace content. |
| `display_name` | String | No | A human-friendly name. |

Each property may appear at most once. A repeated property is reported as
[`AION-004`](cli.md#diagnostic-codes).

### `description`

A mandatory string describing the language. It must contain at least one
non-whitespace character. A missing `description` (or a missing `metadata`
block) is reported as [`AION-003`](cli.md#diagnostic-codes); a description that
is empty or whitespace-only is reported as
[`AION-011`](cli.md#diagnostic-codes).

Both string forms are accepted:

```aion
metadata {
    description = "A short, single-line description.";
}
```

```aion
metadata {
    description = """
        A longer, multiline description that
        preserves its contents verbatim.
    """;
}
```

### `display_name`

An optional string holding a presentation name for the language.

```aion
metadata {
    description = "An ancient, melodic constructed language.";
    display_name = "Aelith";
}
```

## Common errors

| Situation | Code |
|-----------|------|
| No `metadata` block, or no `description` | [`AION-003`](cli.md#diagnostic-codes) |
| `description` is empty or whitespace-only | [`AION-011`](cli.md#diagnostic-codes) |
| `metadata` block or a property repeated | [`AION-004`](cli.md#diagnostic-codes) |
| A property set to a non-string value | [`AION-008`](cli.md#diagnostic-codes) |
