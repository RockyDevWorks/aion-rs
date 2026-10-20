# CLI usage

The `aion` binary is a single-purpose command-line validator.

## Commands

```sh
aion check <file.aion>   # validate a definition
aion --version           # print the version
aion --help              # print help
```

During development you can run the validator from a checkout without
installing it:

```sh
cargo run -- check examples/minimal.aion
```

## Output

A valid file prints:

```
Valid AION definition
```

An invalid file prints a rustc-style diagnostic followed by a summary line. For
example:

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

Each diagnostic carries a stable code, a message, the source filename, a
1-based line and column, a source span, and an optional explanatory note.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Valid definition |
| `1` | Invalid definition |
| `2` | CLI usage error, or file I/O / UTF-8 failure |

## Diagnostic codes

| Code | Meaning |
|------|---------|
| `AION-001` | Syntax error (malformed or unterminated tokens, unknown declarations or properties, invalid identifiers, trailing content, trailing commas) |
| `AION-002` | Unsupported version (only `0.0.1-1` is accepted) |
| `AION-003` | Missing required element (metadata block / description) |
| `AION-004` | Duplicate block or property |
| `AION-008` | Incorrect value type |
| `AION-009` | Integer outside the `u32` range |
| `AION-010` | Duplicate set member |
| `AION-011` | Empty / whitespace-only required value |
| `AION-012` | Invalid Unicode escape (not a valid scalar value) |
| `AION-020` | Input error (file too large, invalid UTF-8, I/O) |

## Limits

- **Maximum input size: 8 MiB.** Larger files are rejected (exit code `2`)
  rather than silently truncated.
- **Maximum diagnostics per run: 100.** Beyond this the validator stops adding
  more.

## Windows note: Application Control (WDAC / AppLocker)

If execution is blocked under a Windows Application Control policy, redirect
Cargo's output directory to an allowed location:

```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\aion-target"
cargo run -- check examples/minimal.aion
```

See [Installation](installation.md#windows-note-application-control-wdac-applocker)
for details.
