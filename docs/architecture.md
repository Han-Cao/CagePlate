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
| `model` | `InputTable` with source positions; `OutputTable` with stable keys, header levels and body cells; normalized `Cage`, `Mouse`, semantic `Genotype`; limits. |
| `error` | Structured `Error`/`Result` with message and optional source row, column and rejected value. |
| `config` | Typed settings, source references, schema validation, defaults path/load/save/recovery. Workflow-specific validation lives beside its algorithm. |
| `tsv` | Quoted TSV codec, shape/size checks, source-column resolution and output presentation. |
| `cage_to_plate` | Inventory → cages → assigned mice → sparse/inline display. |
| `plate_to_cage` | Layout normalization → genotype categories/Cartesian leaves → per-cage summary and grouped headers. |
| `src/bin/cageplate-cli.rs` | Explicit arguments, files and standard I/O. |
| `src/main.rs`, `src/ui/` | Native launch, input/cache state, mappings/settings, preview, clipboard and default controls. |

Workflow APIs are documented in [Cage → Plate](cage-to-plate.md) and [Plate → Cage](plate-to-cage.md); settings APIs are in [configuration](configuration.md).

## Shared TSV contract

- Input is a single-header-row rectangular TSV range; output may have hierarchical headers. Grouped per-cage output is presentation, not another per-mouse input.
- Accept Unicode/UTF-8, an optional BOM, LF/CRLF, quoted tabs/newlines, and doubled quotes inside quoted cells. Reject malformed/unclosed quoting.
- Ignore wholly empty records; require named headers and equal row widths. Preserve trailing empty cells/tabs. Duplicate headers require explicit occurrence mappings.
- Trim structural identifiers, sex, ears, numeric values and genotype tokens. Match headers exactly; preserve free-text remarks, omitting whitespace-only remarks during aggregation.
- A sparse sex label occupies another mouse row's cage cell, not an embedded line break. Layout-specific rules belong to each workflow guide.
- Limits: **100,000** input rows/generated mice, **4,096** columns and **2,000,000** total cells including headers. Check sizes/overflow before expansion or Cartesian allocation; never truncate silently.

Core table utilities in `src/tsv.rs`:

| API | Contract |
| --- | --- |
| `parse_tsv` / `write_tsv` | Parse headered records with physical source lines; serialize every header/body cell with quoting and trailing empties. |
| `InputTable::validate` / `OutputTable::validate` | Check manually constructed table shapes, limits and output-key uniqueness before indexing/rendering. |
| `ColumnRef::resolve` / `from_index` | Resolve a name/duplicate occurrence against current headers, or create a stable mapping from a current selection. |
| `OutputTable::present` / `OutputConfig::validate` | Rename final-row headers and move whole columns; reject duplicate order keys/blank explicit names, ignore stale keys and append new ones. |

## Identity, ordering and state

- Use plain owned records, not a dataframe. Normalize sparse cage/sex cells before grouping; separate raw genotype tokens, semantic states, stable identities and display labels.
- Source mappings use header plus optional duplicate occurrence, never stale numeric positions. Genotype leaf keys sort source-column/state pairs and are independent of display names/hierarchy order. Move full header paths with their data.
- Preserve first-seen cage order and source mouse order; hash maps are lookup-only. Each expanded mouse has one row and each summarized mouse belongs to exactly one bucket, including unknowns. All/Existing is per gene and Existing observations are global within its input column.
- Each GUI workflow caches input, parsed data, preview/TSV, diagnostics and a stale flag. Input/settings changes disable copying until successful reconversion. Preview rows are virtualized, truncated cells have full-text tooltips, and copying always includes the complete table.
- `src/ui/theme.rs` applies an explicit light theme regardless of OS preference, with dark text, readable diagnostic colors, and an opaque root frame. The native clear color matches the panel background; a transparent root would expose a dark background behind light-theme widgets.
- Serialize only `AppConfig`, never animal records, previews, clipboard/history, or telemetry. Defaults save only explicitly; load failures warn without overwriting the original.
- Return structured validation errors rather than guessing layouts, skipping mice, recycling ears, or wrapping numbers. Internal `expect` calls assert already-validated invariants.

## Build boundary

Core dependencies are `csv` (TSV mode), `serde`/`serde_json`, `directories` and `tempfile` (atomic config replacement). Default feature `gui` adds pinned eframe 0.37 with **native `clipboard`**, embedded fonts and Glow, plus X11/Wayland support for optional Linux checks. Do not omit clipboard when disabling eframe defaults. No separate application clipboard integration, WGPU, spreadsheet readers, networking, or database is needed.

The GUI binary requires `gui`; `--no-default-features` builds only core/CLI without display/GPU dependencies. `Cargo.lock` fixes dependency resolution; release builds use size optimization, thin LTO, one codegen unit and stripped symbols. See [development](development.md) for commands, test discipline and the native Windows verification boundary.
