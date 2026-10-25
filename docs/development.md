# Development

Start with this guide and [architecture](architecture.md). Behavior and settings are specified in [Cage → Plate](cage-to-plate.md), [Plate → Cage](plate-to-cage.md), and [configuration](configuration.md).

## Setup

Use Rust **1.95+**, Cargo, rustfmt and Clippy. Builds need crates.io access or a populated cache; the app runs offline. Core/CLI development needs no Excel, display server, or GUI system packages. Disable default features for the headless WSL path.

## Headless CLI

The CLI calls the same library as the GUI and never loads GUI-saved defaults:

```text
cageplate-cli cage-to-plate [--input PATH|-] [--output PATH|-] [--config JSON]
cageplate-cli plate-to-cage [--input PATH|-] [--output PATH|-] [--config JSON]
```

Input/output default to `-` (stdin/stdout). Config is a JSON file path, not stdin; omitted settings use built-ins. Plate → Cage requires a config selecting genotype groups. Use `--help` for usage. Successful stdout contains only TSV; errors go to stderr with nonzero exit status. Output files are overwritten, but only after successful conversion/serialization; write failures may leave partial files. Limits: input 64 MiB, config 1 MiB, plus the table limits in architecture.md.

Run the supplied fixtures from the project root:

```sh
cargo run --locked --no-default-features --bin cageplate-cli -- cage-to-plate --input example/cage2plate.input.txt --config tests/fixtures/cage-to-plate.example.json --output -
cargo run --locked --no-default-features --bin cageplate-cli -- plate-to-cage --input example/plate2cage.input.txt --config tests/fixtures/plate-to-cage.example.json --output -
```

Use `--output PATH` to save TSV, or omit `--input` to pipe data into the CLI.

## Checks and tests

```sh
cargo fmt --check
cargo clippy --locked --no-default-features --all-targets -- -D warnings
cargo test --locked --no-default-features
RUSTDOCFLAGS="-D missing_docs -D rustdoc::broken_intra_doc_links" cargo doc --locked --no-default-features --no-deps
```

If the existing environment supports GUI compilation, also run these; do not install GUI packages merely to run core tests:

```sh
cargo check --locked --features gui --bin cageplate
cargo clippy --locked --features gui --all-targets -- -D warnings
cargo test --locked --features gui --bin cageplate
```

GUI-binary tests check light-theme selection, opaque background and text/diagnostic contrast without opening a window. Module unit tests cover parsing and conversion rules. Integration tests cover golden outputs (`tests/example_fixtures.rs`), invariants/edge cases (`tests/edge_cases.rs`), real CLI processes (`tests/cli.rs`), and temporary-directory persistence (`tests/configuration.rs`). Golden comparisons normalize only CRLF/LF and the final newline—not spaces, empty columns, labels, or ordering.

Initial WSL verification with Rust 1.97.1 passed all **37 tests**, formatting, both Clippy checks, GUI compilation and strict Rustdoc. Both outputs matched: Cage → Plate **19 rows × 6 columns**, ending at `3C`; Plate → Cage **5 rows × 11 columns** with two header levels. **No native Windows executable or GUI/clipboard interaction was tested.** Compilation is not a substitute for the Windows checks below.

## Change checklist (developers and agents)

- Keep conversion rules in the shared library, not UI/CLI adapters. Reuse mapping, validation and presentation utilities; preserve the invariants in architecture.md.
- Add focused tests and keep the four supplied text fixtures unchanged. Plans live under `plans/` when available; that directory is intentionally ignored by the existing repository policy.
- Update public API Rustdoc and the relevant workflow/configuration guide alongside behavior changes. Decide schema compatibility explicitly; unsupported/invalid defaults must not be silently overwritten.
- Test persistence only in temporary directories, never actual user defaults. Avoid global environment/working-directory changes in parallel tests. Never persist or log animal records.
- Keep `Cargo.lock` with dependency changes and recheck the minimum Rust version. Preserve optional GUI dependencies and explicit native `clipboard` support when minimizing eframe features.
- Run the checks above and report what was actually verified, distinguishing WSL/core checks from native Windows behavior.

## Windows build and manual checks

Use a **native Windows** Rust 1.95+ MSVC toolchain with Visual Studio Build Tools (**Desktop development with C++**, MSVC toolset and Windows SDK), then run:

```powershell
cargo build --release --locked --bin cageplate
```

Launch/distribute `target\release\cageplate.exe`.