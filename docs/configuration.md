# Configuration and defaults

## Persistence

**Save as default** explicitly saves `AppConfig` JSON. Startup automatically loads that one file:

- Windows: `%APPDATA%\CagePlate\defaults.json` (standard roaming user config directory).
- Linux/WSL: `$XDG_CONFIG_HOME/CagePlate/defaults.json`, normally `~/.config/CagePlate/defaults.json`.

The GUI displays the actual path. There is no implicit exit/autosave. **Restore built-ins** resets in-memory settings only; click Save as default to persist the reset. Neither pasted records, output tables, previews, clipboard contents nor input history are serialized.

Files have `schema_version: 1`. Missing fields use built-ins; unknown fields, unsupported schema versions and invalid settings are rejected. Missing files are normal. Corrupt/inaccessible/future-version files trigger a visible warning and built-in fallback without overwriting the original. Explicit saving validates first and uses a same-directory temporary file plus atomic replacement. Configs are capped at 1 MiB.

The CLI never reads this implicit GUI file. It uses built-ins or an explicit `--config` path for reproducibility.

## Schema overview

The root contains only `schema_version`, `cage_to_plate`, and `plate_to_cage`. See `src/config.rs` for the complete typed schema and defaults. Both workflow sections may be omitted; partial documents are valid.

Cage → Plate fields:

| Fields | Meaning/default |
| --- | --- |
| `cage`, `quantity`, `sex` | Column mappings: `Cage #`, `QTY`, `SEX`. |
| `plate.rows`, `plate.columns` | 8 × 12; each dimension 1..=1024. |
| `plate.traversal`, `plate.well_style` | `column_wise`, `number_letter`; alternatives `row_wise`, `letter_number`. |
| `plate.start_row`, `plate.start_column` | Zero-based first-well position, default 0/0. GUI shows one-based row/column indices. |
| `plate.start_plate`, `plate.start_mouse` | Positive global starts, default 1/1. |
| `ears` | Empty list = automatic inferred prefix; otherwise explicit nonempty unique labels in assignment order. |
| `layout` | `newline` or `inline`; singleton always inline. |
| `output` | Stable-key ordering and final-header label overrides. |

Plate → Cage fields:

| Fields | Meaning/default |
| --- | --- |
| `cage`, `ear`, `mouse_number` | Required mappings: `Cage`, `Ear`, `#`. |
| `sex`, `within_cage`, `plate`, `well`, `remarks` | Optional mappings, JSON `null` disables each. Defaults: null, `Sample`, `Plate`, `Well`, null. Unmap absent optional columns in the GUI. |
| `layout`, `sex_tokens` | `sparse` (alternatives `inline`, `repeated`), markers `["F","M"]`. Repeated requires separate sex. |
| `groups` | Ordered gene definitions, empty by default until the user selects genes. Conversion requires 1..=8. |
| `tokens` | Four distinct accepted strings: hom `-/-`, het `+/-`, wt `+/+`, unknown `?`. Blank is implicitly unknown. |
| `mouse_label` | `{ear}{mouse_number}`; validated field template. |
| `output` | Stable-key ordering and final-header label overrides. |

Each group has `column`, optional display `name` (blank = source header), `mode` (`existing`/`all`), `order` (default `["hom","het","wt"]`), and `labels` (state-keyed header overrides, such as `{"wt":"WT"}`). Changing tokens or display names does not change semantic states.

## Column references and output identities

A mapping is normally a string (`"Cage"`). Duplicated headers use a zero-based occurrence object:

```json
{ "header": "Gene", "occurrence": 1 }
```

This means the second `Gene`, not the second column in the table. Numeric source positions are never silently reused. A stale/missing/ambiguous saved mapping must be corrected for the current paste.

`output` supports:

```json
{
  "order": ["sex", "cage"],
  "labels": { "cage": "Cage ID", "unknown": "Uncalled" }
}
```

Listed current keys move first; unlisted keys append in natural order; stale keys are ignored. Full hierarchical paths move with leaves. Fixed keys are documented in each workflow guide. A genotype leaf key starts `genotype:` followed by JSON-encoded sorted `(source ColumnRef, semantic state)` pairs; this is generated automatically by the core/GUI, independent of display names and hierarchy order. Per-gene category labels should normally be changed through `groups[].labels`, not opaque leaf keys.

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

`AppConfig::{validate, from_json}` check complete/partial documents and semantics. `ColumnRef::validate` checks source names/occurrences. `default_path` locates but never creates directories; `read_config` explicitly reads a validated file. `load_defaults` returns `LoadedDefaults { config, warning }`, preserving invalid files. `save_defaults` is the only persistence operation and validates before writing. These APIs are tested only in temporary directories, never against the developer's actual defaults.
