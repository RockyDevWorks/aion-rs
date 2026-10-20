# AION

`aion` is a minimal, production-quality command-line validator for the **AION
v0.0.1-1** language-definition format. It reads an `.aion` file and reports
whether it is *structurally* valid according to the AION grammar.

It validates definitions only. It does **not** generate languages or phonemes,
invoke AI, produce ELANG output, interpret linguistic meaning, or modify source
files.

## What AION looks like

The smallest valid definition:

```aion
aion 0.0.1-1;

language aelith {
    metadata {
        description = "An ancient, melodic constructed language.";
    }
}
```

A fuller example exercising every supported section lives in
[`examples/complete.aion`](examples/complete.aion).

## Supported definitions

- **Document**: a single `aion 0.0.1-1;` version declaration followed by one
  `language <identifier> { ... }` declaration.
- **Metadata** (mandatory, exactly one):
  - `description` — mandatory string with non-whitespace content.
  - `display_name` — optional string.
- **Intent phonology** (optional `intent phonology { ... }`, at most once):
  `description` (string), and `style`, `character`, `articulation`, `rhythm`,
  `avoid` (string or set of strings), plus `references` (set of typed
  references via `language("...")` / `conlang("...")`).
- **Phonology inventory** (optional `phonology.inventory { ... }`, at most
  once), with `consonants` and `vowels` blocks (each at most once), supporting
  `cardinality` (u32), `include` / `exclude` (sets of phoneme literals such as
  `/m/`, `/ɳ/`, `/ə/`).

Language sections may appear in any order; each may appear at most once, and
each property at most once within its block.

## Building

Requires a stable Rust toolchain (built and tested with Rust 1.99 /
edition 2021).

```sh
cargo build            # debug build
cargo build --release  # optimized build
```

## Usage

```sh
aion check <file.aion>   # validate a definition
aion --version           # print the version
aion --help              # print help
```

Example:

```sh
cargo run -- check examples/minimal.aion
```

### Output

Valid input prints:

```
Valid AION definition
```

Invalid input prints a rustc-style diagnostic followed by a summary, for
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

### Exit codes

| Code | Meaning                                   |
|------|-------------------------------------------|
| `0`  | Valid definition                          |
| `1`  | Invalid definition                        |
| `2`  | CLI usage error or file I/O / UTF-8 failure |

## Testing

```sh
cargo test                               # unit + conformance tests
cargo fmt --check                        # formatting
cargo clippy --all-targets -- -D warnings  # lints (warnings are errors)
```

Fixtures live under `tests/fixtures/valid/` and `tests/fixtures/invalid/`;
`tests/validation.rs` drives them and asserts the expected diagnostic codes.

### Windows note: Application Control (WDAC / AppLocker)

If your checkout lives under a directory covered by a Windows Application
Control policy (for example, parts of the Desktop on some managed machines),
the OS can block execution of binaries written there (`os error 4551`) or the
linker from writing them (`LNK1104`). Redirect Cargo's output directory to a
location the policy allows, then build/test/run from there:

```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\aion-target"
cargo test
cargo run -- check examples/minimal.aion
```

This does not affect the source tree; only build artifacts move.

## Continuous integration

CI runs on GitHub Actions (`.github/workflows/`):

- **`ci.yml`**
  - `rustfmt` — `cargo fmt --all --check`.
  - `clippy` — `cargo clippy --all-targets --all-features -- -D warnings`.
  - `test` — build + `cargo test` on Linux, Windows, and macOS using a pinned
    recent stable toolchain (**Rust 1.99.1**), plus a pinned **MSRV 1.85.0**
    build on Linux, and doc tests.
  - `doc` — `cargo doc --no-deps` with `-D warnings`.
  - `semver-checks` — [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks)
    on pull requests, to flag breaking changes to the library API.
- **`audit.yml`** — `cargo audit` for known security advisories, on dependency
  changes and on a weekly schedule.

Dependencies and the actions themselves are kept current by **Dependabot**
(`.github/dependabot.yml`): weekly updates for the `cargo` and
`github-actions` ecosystems, with routine minor/patch cargo bumps grouped into
a single pull request.

The primary CI toolchain is pinned to a recent stable release (**Rust 1.99.1**)
via the `RUST_STABLE` env var in `ci.yml`; bump it as new releases land. The
minimum supported Rust version (MSRV) is **1.85.0**, recorded as `rust-version`
in `Cargo.toml`. 1.85 is required because transitive dependencies (e.g.
`clap_lex`) ship edition-2024 manifests, which older Cargo cannot parse.

## Diagnostics

Each diagnostic carries a stable code, a message, the source filename, a
1-based line and column, a source span, and an optional explanatory note.

| Code       | Meaning                                              |
|------------|------------------------------------------------------|
| `AION-001` | Syntax error (malformed/unterminated tokens, unknown declarations or properties, invalid identifiers, trailing content, trailing commas) |
| `AION-002` | Unsupported version (only `0.0.1-1` is accepted)     |
| `AION-003` | Missing required element (metadata block / description) |
| `AION-004` | Duplicate block or property                          |
| `AION-008` | Incorrect value type                                 |
| `AION-009` | Integer outside the u32 range                        |
| `AION-010` | Duplicate set member                                 |
| `AION-011` | Empty / whitespace-only required value               |
| `AION-012` | Invalid Unicode escape (not a valid scalar value)    |
| `AION-020` | Input error (file too large, invalid UTF-8, I/O)     |

### Duplicate-member comparison policy

- **String sets** and **reference sets**: members are compared by their
  *decoded* value (escapes resolved). References also compare the kind
  (`language` vs `conlang`).
- **Phoneme sets**: literals are compared by their exact literal content
  between the delimiting slashes, with no normalization.

The source is never normalized or rewritten.

## Limits

- Maximum input size: **8 MiB**. Larger files are rejected (exit code 2) rather
  than silently truncated.
- Maximum diagnostics collected per run: **100**. Beyond this the validator
  stops adding more.

## Project layout

```
aion-rs/
├── Cargo.toml
├── README.md
├── grammar/
│   └── aion.pest          # authoritative grammar (compiled into the crate)
├── src/
│   ├── main.rs            # CLI: argument parsing, rendering, exit codes
│   ├── lib.rs             # library entry point (`check`)
│   ├── parser.rs          # Pest wrapper -> parse tree or syntax diagnostic
│   ├── validator.rs       # structural rules -> diagnostics
│   └── diagnostic.rs      # diagnostic types, codes, spans, rendering
├── tests/
│   ├── validation.rs
│   └── fixtures/{valid,invalid}/
└── examples/
    ├── minimal.aion
    ├── complete.aion
    └── invalid.aion
```

### Grammar authority

`grammar/aion.pest` is the single authoritative grammar, compiled into the
crate. See the header in that file for the corrections applied to the original
grammar.

## License

Dual-licensed under MIT or Apache-2.0.
