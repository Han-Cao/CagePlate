# Configuration and defaults

## Persistence

The tool's built-ins (`Default` implementations) and the user's saved defaults are distinct. Startup loads user `AppConfig` JSON from one file, falling back to built-ins if absent/invalid:

- Windows: `%APPDATA%\CagePlate\defaults.json` (standard roaming user config directory).
- Linux/WSL: `$XDG_CONFIG_HOME/CagePlate/defaults.json`, normally `~/.config/CagePlate/defaults.json`.

Each workflow's **Config** pop-up uses the app's light-gray background so white single-line and multiline text editors remain visibly distinct from their surroundings. It displays the actual path and edits a private draft. **Apply** validates/activates only that draft and closes; **Cancel**, Escape, or backdrop dismissal discards it. **Reset** stages only that workflow's tool built-ins, not the user's saved defaults; Apply activates without saving. **Save as default** validates, merges only this workflow into the latest valid file, saves atomically, then applies it and keeps the pop-up open. The other workflow's saved values are preserved, never replaced with its unsaved edits. Cancel after a successful save discards only subsequent edits. For Plate to Cage, selected genotype source mappings and generated `genotype:` output identities are transient and are omitted from the saved workflow; each selected genotype's display/category settings are instead appended or updated in `genotype_display_defaults` under a normalized source-name key.

There is no implicit exit/autosave. Draft-only edits do not stale previews; input/header-mode changes and changed applied settings do. Neither pasted records, output tables, previews, clipboard contents nor input history are serialized.

Files have `schema_version: 1`. Missing fields use built-ins; unknown fields, unsupported schema versions and invalid settings are rejected. Missing files are normal. Corrupt/inaccessible/future-version files trigger a visible warning and built-in fallback without overwriting the original. Saving validates first and uses a same-directory temporary file plus atomic replacement. GUI workflow-scoped saving refuses to overwrite an unreadable, corrupt, or unsupported existing file; fix/recover that file explicitly before saving. Failed saves leave active settings unchanged and the draft available for correction. Configs are capped at 1 MiB.

Schema version remains 1: old partial files still load and omitted `has_header` fields mean true. Plate to Cage now requires `has_header: true`; old files that explicitly set it false are rejected with a visible recovery warning and preserved on disk. Correct that field and remap real header/gene names before using such a file. Older app versions reject new fields/forms under their strict unknown-field policy; do not use newly saved defaults with an older binary without removing unsupported settings.

The CLI never reads this implicit GUI file. It uses built-ins or an explicit `--config` path for reproducibility.

## Schema overview

The root contains only `schema_version`, `cage_to_plate`, and `plate_to_cage`. See `src/config.rs` for the complete typed schema and defaults. Both workflow sections may be omitted; partial documents are valid.

Cage → Plate fields:

| Fields | Meaning/default |
| --- | --- |
| `has_header` | `true` initially; the cage GUI infers it after a paste, with manual checkbox override. False treats every nonempty record as data. CLI mode stays explicit. |
| `cage`, `quantity`, `sex` | Column mappings: `Cage #`, `QTY`, `SEX`. |
| `plate.rows`, `plate.columns` | 8 × 12; each dimension 1..=1024. |
| `plate.traversal`, `plate.well_style` | `column_wise`, `number_letter`; alternatives `row_wise`, `letter_number`. |
| `plate.start_row`, `plate.start_column` | Zero-based first-well position, default 0/0. GUI shows one-based row/column indices. |
| `plate.start_plate`, `plate.start_mouse` | Positive global starts, default 1/1. |
| `ears` | Empty list = automatic inferred prefix; otherwise explicit nonempty unique labels in assignment order. |
| `layout` | `newline` (Sex on a new line), `inline` (Inline sex), or `separate_column` (Separate column). Singletons use inline fallback only in the first two layouts. |
| `output` | Stable-key ordering and final-header label overrides. |

Plate → Cage fields:

| Fields | Meaning/default |
| --- | --- |
| `has_header` | Must be `true`; false is rejected because source/genotype headers are required. |
| `cage`, `ear`, `mouse_number` | Required mappings: `Cage`, `Ear`, `#`. |
| `sex`, `within_cage`, `plate`, `well`, `remarks` | Optional mappings, JSON `null` disables each. Defaults: null, `Sample`, `Plate`, `Well`, null. Unmap absent optional columns in the GUI. |
| `layout`, `sex_tokens` | `sparse` (Sex on a new line), `inline` (Inline sex), or `repeated` (Separate column). Markers `["F","M"]`. Repeated requires separate sex. |
| `groups` | Ordered gene definitions, empty by default until the user selects genes. Conversion requires 1..=8. |
| `tokens` | Four distinct accepted strings: hom `-/-`, het `+/-`, wt `+/+`, unknown `?`. Blank is implicitly unknown. |
| `mouse_label` | `{ear_label}{global_id}`; validated field template. Allowed fields are `{cage_id}`, `{within_cage_id}`, `{ear_label}`, `{global_id}`, and `{sex}`. |
| `genotype_display_defaults` | Normalized source genotype names mapped to remembered display/category settings; selected source columns are not stored here. |
| `output` | Stable-key ordering and final-header label overrides. Dynamic `genotype:` entries are transient during scoped Plate to Cage default saves. |

Each group has `column`, optional display `name` (blank = source header or `Column N`), `mode` (`existing`/`all`), `order` (default `["hom","het","wt"]`), and `labels` (state-keyed header overrides, such as `{"wt":"WT"}`). Changing tokens or display names does not change semantic states. The saved display profile contains the group presentation fields but not `column`; selecting a source with the same normalized name reloads that profile.

## Column references and output identities

A mapping is normally a string (`"Cage"`). Duplicated headers use a zero-based occurrence object:

```json
{ "header": "Gene", "occurrence": 1 }
```

This means the second `Gene`, not the second column in the table. Headerless mappings instead use an explicit **one-based** column object:

```json
{ "column": 3 }
```

This is always the third column, checked against the current width. Valid numbers are 1..=4096; source reordering requires reviewing positional mappings. Synthetic `Column N` labels are display-only and never resolve named references. Headered GUI selections remain name/occurrence based; headerless selections are positional. Switching modes preserves text but does not guess conversions of old mappings: correct missing/incompatible mappings in Config. The GUI uses selection lists in both cases: source names with headers, **Column N** without headers. Before a paste, the lists are disabled because no source columns are known. Disable absent optional sources with **Not used**. Name-based suggestions are disabled for headerless cage data; Plate to Cage does not allow headerless data.

For headerless cage inventory columns cage/count/sex, use:

```json
{
  "cage_to_plate": {
    "has_header": false,
    "cage": { "column": 1 },
    "quantity": { "column": 2 },
    "sex": { "column": 3 }
  }
}
```

In Cage to Plate, each paste updates the checkbox from the first nonempty TSV record: a finite numeric cell (including trimmed, signed, decimal or scientific notation) means headerless; otherwise headered. Digits inside `Cage1` or `5XFAD` are not numbers. The checkbox remains manually editable; ordinary typing does not rerun the heuristic. Inference never guesses source mappings. Plate to Cage hides the option and always requires headers. The CLI uses explicit JSON rather than this paste heuristic or implicit user defaults.

Both previews retain headers. **Copy output (with headers)** copies them; **Copy output (without headers)** omits every header level. Both are disabled while output is absent/stale.

Cage Config sections are **Input columns** and **Output format**, with **Cage sex layout**, **Plate layout**, **Ear labels**, and **Output columns**. Output field meanings (Cage ID, Within-cage ID, Ear label, Plate ID, Well ID, Global ID) appear before the editable default names (`Cage`, `Sample`, `Ear`, `Plate`, `Well`, `#`). Separate column adds `SEX` immediately after Cage. Per-field Reset name controls and confirmation text are removed; the workflow's Reset still restores all tool defaults.

Plate Config's **Input columns** contains Cage ID, Within-cage ID, Ear label, Plate ID, Well ID, Global ID, Remarks, Sex, then **Genotype (level 1)**. **Add genotype** appears below the final genotype row and adds another level selector; move/remove controls operate on these input rows. The **Genotypes** section automatically shows a card per selected gene with **Include genotypes** and **Display names**, followed by **Accepted genotype values**. A newly added blank level must be selected or removed before Apply/save. The output-column preview is rebuilt from the current draft/table so genotype headers follow live display edits.

`output` supports:

```json
{
  "order": ["sex", "cage"],
  "labels": { "cage": "Cage ID", "unknown": "Uncalled" }
}
```

Listed current keys move first; unlisted keys append in natural order; stale keys are ignored. Full hierarchical paths move with leaves. Fixed keys are documented in each workflow guide. A genotype leaf key starts `genotype:` followed by JSON-encoded sorted `(source ColumnRef, semantic state)` pairs; this is generated automatically by the core/GUI, independent of display names and hierarchy order. Per-gene category labels should normally be changed through `groups[].labels`, not opaque leaf keys. These generated keys and their output labels/order entries are not persisted by a scoped Plate to Cage default save.

## Example-compatible configuration

`tests/fixtures/cage-to-plate.example.json` selects the cage fixture mappings. The genotype fixture uses:

```json
{
  "schema_version": 1,
  "plate_to_cage": {
    "remarks": "Remarks",
    "groups": [
      { "column": "5XFAD", "mode": "existing" },
      { "column": "LSL", "mode": "all", "labels": { "wt": "WT" } }
    ]
  }
}
```

## Configuration APIs

`AppConfig::{validate, from_json}` check complete/partial documents and semantics. `ColumnRef::validate` checks source names/occurrences/positions; `header()` returns an optional source name and `display_name()` provides a human-readable `Column N` fallback. `default_path` locates but never creates directories; `read_config` explicitly reads a validated file. `load_defaults` returns `LoadedDefaults { config, warning }`, preserving invalid files. `save_defaults` validates and writes a complete config; `save_workflow_defaults(path, WorkflowDefaults::CageToPlate(...))` (or `PlateToCage`) merges a single workflow into the latest valid saved config and reuses the atomic writer. All persistence tests use temporary directories, never the developer's actual defaults.
