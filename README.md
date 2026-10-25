# CagePlate

A lightweight, offline Rust + egui Windows app for mouse cage inventories and DNA sample plates. Paste a spreadsheet table, map its columns, configure the conversion, then copy the result back. No Excel installation, spreadsheet file reader, database, or network service is used by the app.

## Workflows

- **Cage → Plate:** expand cage counts to one mouse per row, assign ears and sequential wells/plates, and render sex on a new line, inline sex, or a separate sex column. Plate geometry, fill direction, first well, numbering, ear order, and output column names/order are configurable.
- **Plate → Cage:** normalize sparse, inline, or repeated mouse rows; summarize any ordered hierarchy of selected genotype columns. Each gene supports **All** or **Existing** categories, custom genotype values/header names, and configurable mouse labels. Unknown calls are collected in one `?` column.
- **Config:** each workflow has its own settings pop-up with **Apply / Cancel**, **Save as default**, and **Reset**. Save user defaults independently; Reset stages the tool's built-ins. Pasted records and generated results are never saved with defaults. Output cells are read-only.

GUI I/O is **clipboard TSV only**. The text examples are automated test fixtures, not file-import requirements. `example/test.xlsx` is intentionally unused.

## Build the Windows app

Use native Windows, Rust **1.95+**, and the MSVC C++ build tools/Windows SDK:

```powershell
cargo build --release --locked --bin cageplate
```

Launch `target\release\cageplate.exe`. The app embeds its fonts and needs no adjacent data files. It uses the system OpenGL driver; verify the executable on the intended Windows computer. See [development and Windows checklist](docs/development.md).

## Use the GUI

1. Choose **Cage to Plate** or **Plate to Cage**.
2. In **Input**, click **Paste / replace**, or focus the box and press Ctrl+V. **Cage to Plate** updates **Input includes header row** after each paste: any numeric cell in the first nonempty record means no header; otherwise it assumes a header. You can correct the checkbox manually. **Plate to Cage requires headers**, including genotype names, and has no headerless option.
3. Open **Config → Input columns**. Select source names when headers exist; for headerless cage inventories, select **Column 1**, **Column 2**, etc. from the lists. No column numbers need to be typed. Missing/incompatible mappings require explicit remapping. In Plate to Cage, select **Genotype (level 1)** and use **Add genotype** for additional levels.
4. Use **Output format**, **Plate layout**, **Ear labels**, **Output columns**, or each gene's **Genotypes → Display names** controls. Sections use a bundled bold font. **Apply** activates changes and closes; **Cancel**, Escape, or clicking outside discards unapplied edits. Only input/applied-setting changes disable copying until reconversion.
5. Click **Convert**, review **Output**, then choose **Copy output (with headers)** or **Copy output (without headers)**. The latter omits every output header level, not data rows.
6. In Config, **Save as default** saves and applies only this workflow and keeps the pop-up open. It preserves the other workflow's saved values, not its unsaved edits. Startup uses user defaults when available. **Reset** stages only this workflow's tool defaults; Apply activates the reset without saving, or Save as default persists it. Cancel never undoes a successful save.

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

Section titles embed Ubuntu Bold (Copyright 2011 Canonical Ltd.), under the [Ubuntu Font Licence](assets/fonts/UFL.txt); see [font attribution](assets/fonts/README.md). The app still runs offline with no adjacent font/data files.
