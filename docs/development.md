# Development

Start with this guide and [architecture](architecture.md). Behavior and settings are specified in [Cage → Plate](cage-to-plate.md), [Plate → Cage](plate-to-cage.md), and [configuration](configuration.md).

## Setup

Use Rust **1.95+** with edition **2024**, Cargo, rustfmt and Clippy. `Cargo.toml` must declare the edition explicitly; otherwise Cargo defaults to 2015, causing modern imports (including serde) to fail. For WSL editor diagnostics, open/reload the workspace with the editor's WSL rust-analyzer and check its project-load log. Absence of a shell `rust-analyzer` binary does not prove the editor's bundled server is missing. Builds need crates.io access or a populated cache; the app runs offline. Core/CLI development needs no Excel, display server, or GUI system packages. Disable default features for the headless WSL path.

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

For headerless **Cage to Plate** input, set `has_header: false` in that workflow's explicit JSON and map every source using `{ "column": N }` (one-based). See [configuration](configuration.md#column-references-and-output-identities) for a minimal cage/count/sex config. For example, after saving that example as `headerless.json`:

```sh
printf 'A\t2\tF\n' | cargo run --locked --no-default-features --bin cageplate-cli -- cage-to-plate --config headerless.json
```

No separate header-mode CLI flag, paste heuristic, or implicit user defaults are used in the CLI. The first headerless cage record remains data. Plate to Cage requires headers and rejects `has_header: false`. CLI output includes headers; GUI clipboard buttons can include them or omit every header level.

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

GUI-binary tests check light-theme/background/contrast, modal draft isolation and Apply/Cancel/Reset/save failures, header-mode inference/reparsing, headerless column selection, level-by-level genotype selection/cards, real bold heading font selection, both copy-button caches/stale gating, Escape dismissal, and vector-arrow rendering/click/keyboard/disabled responses without opening a window. Module unit tests cover parsing and conversion rules. Integration tests cover golden outputs (`tests/example_fixtures.rs`), invariants/edge cases (`tests/edge_cases.rs`), real CLI processes (`tests/cli.rs`), and temporary-directory persistence (`tests/configuration.rs`). Golden comparisons normalize only CRLF/LF and the final newline—not spaces, empty columns, labels, or ordering.

Golden outputs remain Cage → Plate **19 rows × 6 columns**, ending at `3C`, and Plate → Cage **5 rows × 11 columns** with two header levels. Tests derive equivalent headerless Cage to Plate inputs and reject headerless Plate to Cage inputs without modifying supplied fixtures. **WSL/headless checks do not verify native Windows rendering, real editor diagnostics, or GUI/clipboard interaction.** Compilation is not a substitute for the checks below.

## Verified for the GUI refinement update

Rust/Cargo 1.97.1 passed the WSL core/CLI and headless GUI suites (**70 tests** including Rustdoc), formatting, core/GUI Clippy with warnings denied, GUI compilation, and strict Rustdoc. Coverage includes paste inference/manual override, dropdown source mapping, genotype-level selection/cards, real bold section fonts, both clipboard modes/stale gating, and separate-sex-column round trips. Native Windows MSVC release compilation and **69 tests** also passed; the difference is the Unix-only permissions test. The updated executable is `target\release\cageplate.exe` (the older isolated-target build is not updated by this command).

These checks do **not** certify the live editor's displayed diagnostics, visible native arrow rendering, real desktop/Excel clipboard interaction, or the full Windows manual checklist. Reopen/reload rust-analyzer if stale editor warnings persist, and complete the native checks below.

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

Launch/distribute `target\release\cageplate.exe`. Windows locks a running executable; close it before overwriting that path, or leave it running and build a separate artifact:

```powershell
cargo build --release --locked --offline --target x86_64-pc-windows-msvc --bin cageplate
```

The isolated executable is `target\x86_64-pc-windows-msvc\release\cageplate.exe`.

- Inspect the two workflow tabs without a preceding app-name heading, bold section/collapsible/card titles, and reorder-arrow controls at normal/high DPI. Arrows are painted vectors; titles use embedded Ubuntu Bold, with its licence/attribution under `assets/fonts/`. Check that editable text controls are white like the main input editor and output meanings are right-aligned with colons.
- At the minimum 720 × 560 window size, open both Config pop-ups, scroll through every section and confirm Apply/Cancel/Save as default/Reset remain reachable. Background controls must be blocked; Escape should close an open combo-box before discarding the modal.
- Check that draft edits and Cancel/backdrop/Escape leave active settings and copyable output untouched. Apply affects only its workflow; Reset stages only its tool defaults. A successful Save applies and updates the saved baseline; a failed Save retains the draft without applying.
- Save each workflow with unrelated unsaved changes in the other, restart and verify only the selected defaults persisted. Corrupt/unsupported existing defaults must warn and refuse scoped saving without overwriting the file.
- Paste headered/headerless cage inventories and verify the numeric-cell inference rule, manual checkbox override, first data row and Column N selection lists. Plate to Cage must have no headerless option. Test Input columns order, Genotype (level 1), Add genotype below the final row, input-row moves/removal, matching cards, normalized genotype display-profile reuse, and the Mouse ID/Allowed sex controls.
- Test all three cage/sex layouts, including singleton and multi-mouse separate-column output feeding Plate to Cage. Confirm output field meanings and unchanged default names. Round-trip both copy buttons through Excel; body-only copying must omit every hierarchical header row. Changes should disable both copy buttons until reconversion.