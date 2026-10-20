# Installation

The `aion` validator is a Rust crate. There is no prebuilt binary to download;
you build it from source with Cargo.

## Prerequisites

- A stable Rust toolchain. The project is built and tested with
  **Rust 1.99.1**, and the minimum supported Rust version (MSRV) is
  **1.74.0**, recorded as `rust-version` in `Cargo.toml`.
- Edition 2021.

If you do not have Rust installed, use [rustup](https://rustup.rs/).

## Build from source

Clone the repository and build with Cargo:

```sh
git clone https://github.com/RockyDevWorks/aion-rs.git
cd aion-rs
cargo build            # debug build
cargo build --release  # optimized build
```

The compiled binary is named `aion` and is written under the Cargo target
directory (`target/debug/aion` or `target/release/aion`).

## Run without installing

During development you can run the validator straight from the checkout:

```sh
cargo run -- check examples/minimal.aion
```

## Install the binary

To place `aion` on your `PATH`, install it from the checkout:

```sh
cargo install --path .
```

## Windows note: Application Control (WDAC / AppLocker)

If your checkout lives under a directory covered by a Windows Application
Control policy (for example, parts of the Desktop on some managed machines),
the OS can block execution of binaries written there (`os error 4551`) or the
linker from writing them (`LNK1104`). Redirect Cargo's output directory to a
location the policy allows, then build, test, and run from there:

```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\aion-target"
cargo test
cargo run -- check examples/minimal.aion
```

This does not affect the source tree; only build artifacts move.

## Verify the install

```sh
aion --version
aion check examples/minimal.aion
```

A valid file prints `Valid AION definition` and exits with code `0`. See
[CLI usage](cli.md) for the full list of commands and exit codes.
