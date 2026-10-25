# CagePlate

A lightweight, offline Rust + egui Windows app for mouse cage inventories and DNA sample plates. Paste a spreadsheet table, map its columns, configure the conversion, then copy the result back. No Excel installation, spreadsheet file reader, database, or network service is used by the app.

## Workflows

- **Cage → Plate:** expand cage counts to one mouse per row, assign ears and sequential wells/plates, and render sparse cage/sex labels or `Cage (Sex)`. Plate geometry, fill direction, first well, numbering, ear order, and output column names/order are configurable.
- **Plate → Cage:** normalize sparse, inline, or repeated mouse rows; summarize any ordered hierarchy of selected genotype columns. Each gene supports **All** or **Existing** categories, custom genotype values/header names, and configurable mouse labels. Unknown calls are collected in one `?` column.
- **Save as default:** save settings explicitly and load them at next launch. Pasted mouse records and generated results are never saved with defaults. Output cells are read-only; column names/order are changed in settings.

GUI I/O is **clipboard TSV only**. The text examples are automated test fixtures, not file-import requirements. `example/test.xlsx` is intentionally unused.

## Build the Windows app

Use native Windows, Rust **1.95+**, and the MSVC C++ build tools/Windows SDK:

```powershell
cargo build --release --locked --bin cageplate
```

Launch `target\release\cageplate.exe`. The app embeds its fonts and needs no adjacent data files. It uses the system OpenGL driver; verify the executable on the intended Windows computer. See [development and Windows checklist](docs/development.md).

## Use the GUI

1. Choose **Cage → Plate** or **Plate → Cage**.
2. Copy a headered range from Excel and click **Paste / replace**, or focus the input box and press Ctrl+V.
3. Select source columns. **Suggest common column mappings** is optional and explicit; missing saved mappings must be corrected.
4. Configure plate settings or add/reorder genotype groups. Expand the names/order controls to customize headers.
5. Click **Convert**, review the table, and **Copy full output (with headers)**. Changing input/settings disables copying until reconversion.
6. Choose **Save as default** if these settings should be loaded next time. **Restore built-ins** changes in-memory settings; save again to persist the reset.

To reproduce the supplied genotype example, add `5XFAD` first with **Existing**, then `LSL` with **All**; override the `LSL` WT header to `WT` and map `Remarks`.

## WSL / headless development

```sh
cargo test --locked --no-default-features
```

See [development](docs/development.md#headless-cli) for CLI usage and fixture commands. The CLI uses built-ins or an explicit JSON config, never GUI-saved defaults. WSL checks do **not** verify native Windows GUI/clipboard behavior.

## Documentation

- [Cage → Plate](docs/cage-to-plate.md) · [Plate → Cage](docs/plate-to-cage.md)
- [Configuration](docs/configuration.md)
- [Architecture and TSV contract](docs/architecture.md) · [Development, CLI and verification](docs/development.md)

Public core APIs also have Rustdoc: `cargo doc --no-default-features --no-deps --open`.
