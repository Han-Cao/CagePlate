# Architecture

## Pipeline and modules

```text
clipboard ─ egui ─┐
                  ├─ parse/map ─ normalized records ─ transform ─ OutputTable ─ TSV
files/stdin ─ CLI ┘
```

GUI and CLI share deterministic library transformations. Conversion receives settings as arguments and never reads files, clipboard, environment, time, or global state. Configuration persistence is a separate explicit operation.

| Module | Responsibility |
| --- | --- |
| `model` | `InputTable` with source-header mode, real/synthesized column labels and source positions; `OutputTable` with stable keys, header levels and body cells; normalized `Cage`, `Mouse`, semantic `Genotype`; limits. |
| `error` | Structured `Error`/`Result` with message and optional source row, column and rejected value. |
| `config` | Typed settings, source references, schema validation, defaults path/load/save/recovery. Workflow-specific validation lives beside its algorithm. |
| `tsv` | Quoted TSV codec, shape/size checks, source-column resolution and output presentation. |
| `cage_to_plate` | Inventory → cages → assigned mice → new-line, inline, or separate-sex-column display. |
| `plate_to_cage` | Layout normalization → genotype categories/Cartesian leaves → per-cage summary and grouped headers. |
| `src/bin/cageplate-cli.rs` | Explicit arguments, files and standard I/O. |
| `src/main.rs`, `src/ui/` | Native launch, workflow input/cache state, preview/clipboard, per-workflow Config modals and source mappings. |
| `src/ui/config_dialog.rs` | Typed workflow drafts, Apply/Cancel/Reset/save actions, alternative-mode mapping view and scoped commits. |

Workflow APIs are documented in [Cage → Plate](cage-to-plate.md) and [Plate → Cage](plate-to-cage.md); settings APIs are in [configuration](configuration.md).

## Shared TSV contract

- The shared TSV parser uses an explicit `has_header` flag (default true). The Cage to Plate GUI alone infers that option after a paste from numeric cells in the first nonempty record, with manual override; CLI parsing remains explicit. Headerless cage input retains its first record as data and synthesizes display-only `Column N` labels. Plate to Cage rejects headerless tables/configs because genotype headers are required. Output tables retain headers and may have hierarchical headers; body-only clipboard serialization can omit them. Grouped per-cage output is presentation, not another per-mouse input.
- Accept Unicode/UTF-8, an optional BOM, LF/CRLF, quoted tabs/newlines, and doubled quotes inside quoted cells. Reject malformed/unclosed quoting.
- Ignore wholly empty records; require nonempty names when source headers are present and equal row widths in both modes. Preserve trailing empty cells/tabs. Duplicate headers require explicit occurrence mappings.
- Trim structural identifiers, sex, ears, numeric values and genotype tokens. Match headers exactly; preserve free-text remarks, omitting whitespace-only remarks during aggregation.
- A sparse sex label occupies another mouse row's cage cell, not an embedded line break. Layout-specific rules belong to each workflow guide.
- Limits: **100,000** input rows/generated mice, **4,096** columns and **2,000,000** total cells including actual/synthesized header labels. Check sizes/overflow before expansion or Cartesian allocation; never truncate silently.

Core table utilities in `src/tsv.rs`:

| API | Contract |
| --- | --- |
| `parse_tsv` / `parse_tsv_with_header` / `infer_has_header` | Preserve the headered API, explicit-mode parsing and physical lines; infer only the cage GUI's checkbox from first-record numeric cells. |
| `write_tsv` / `write_tsv_without_headers` | Serialize all output header/body cells, or body rows only, preserving quoting, ordering and trailing empties. |
| `InputTable::validate` / `OutputTable::validate` | Check manually constructed table shapes, limits and output-key uniqueness before indexing/rendering. |
| `ColumnRef::resolve` / `from_index` | Resolve names/occurrences or explicit one-based positions; reject names against headerless data. Construct names/occurrences for headered selections and positions for headerless selections. |
| `OutputTable::present` / `OutputConfig::validate` | Rename final-row headers and move whole columns; reject duplicate order keys/blank explicit names, ignore stale keys and append new ones. |

## Identity, ordering and state

- Use plain owned records, not a dataframe. Normalize sparse cage/sex cells before grouping; separate raw genotype tokens, semantic states, stable identities and display labels.
- Headered GUI source mappings use header plus optional duplicate occurrence; headerless mappings use explicit one-based positions, checked against current width. Never silently convert missing names to positions or treat synthesized labels as header identities. Genotype leaf keys sort source-column/state pairs and are independent of display names/hierarchy order. Move full header paths with their data.
- Preserve first-seen cage order and source mouse order; hash maps are lookup-only. Each expanded mouse has one row and each summarized mouse belongs to exactly one bucket, including unknowns. All/Existing is per gene and Existing observations are global within its input column.
- Each GUI workflow caches input, parsed data, preview/TSV (with and without headers), diagnostics and a stale flag. Input/header-mode changes and changed applied settings disable copying until successful reconversion; editing a private Config draft does not. Apply validates/commits only that workflow, Cancel/Escape/backdrop discards, Reset stages only its tool defaults, and Save atomically persists before committing. A Reset-mode draft uses a separate parsed mapping view without touching live caches. Preview rows are virtualized, truncated cells have full-text tooltips, and both copy buttons always include every body row. Body-only copying omits all hierarchy header rows, never data.
- `src/ui/theme.rs` embeds Ubuntu Bold as a dedicated section-title font family (licence in `assets/fonts/UFL.txt`); `RichText::strong()` alone only changes color. Section/collapsible/card titles use this real bold face while normal text retains the default fonts. It also applies an explicit light theme regardless of OS preference, with dark text, readable diagnostic colors, and an opaque root frame. The native clear color matches the panel background; a transparent root would expose a dark background behind light-theme widgets.
- Essential reorder arrows use egui's `Painter::arrow` behind normal accessible buttons, not font glyphs. The app header contains only the two plain-text workflow tabs; output field meanings precede right-aligned editable names with a trailing colon, without per-field reset/confirmation controls. Headered and headerless cage mappings both use source selection lists, never numeric text entry. Plate gene selection/add/reorder/removal lives in Input columns; the Genotypes cards edit only category/display settings, and all editable text controls use the same white extreme-background fill as the main input editor.
- Serialize only `AppConfig`, never animal records, previews, clipboard/history, or telemetry. Defaults save only explicitly. Workflow-scoped saves merge into the latest valid file and preserve the other saved section; Plate to Cage scoped saves omit transient selected genotype source/output identities while retaining normalized genotype display profiles; load failures warn, and scoped-save failures preserve the original file and live settings.
- Return structured validation errors rather than guessing layouts, skipping mice, recycling ears, or wrapping numbers. Internal `expect` calls assert already-validated invariants.

## Build boundary

The package explicitly declares Rust edition 2024 (minimum Rust 1.95). Omitting the edition defaults Cargo to 2015 and breaks the modern external-crate imports/project model. Core dependencies are `csv` (TSV mode), `serde`/`serde_json`, `directories` and `tempfile` (atomic config replacement). Default feature `gui` adds pinned eframe 0.37 with **native `clipboard`**, embedded fonts and Glow, plus X11/Wayland support for optional Linux checks. Do not omit clipboard when disabling eframe defaults. No separate application clipboard integration, WGPU, spreadsheet readers, networking, or database is needed.

The GUI binary requires `gui`; `--no-default-features` builds only core/CLI without display/GPU dependencies. `Cargo.lock` fixes dependency resolution; release builds use size optimization, thin LTO, one codegen unit and stripped symbols. See [development](development.md) for commands, test discipline and the native Windows verification boundary.
