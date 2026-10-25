# Cage → Plate

## Contract

Input is a rectangular cage inventory with mapped **cage ID**, **positive integer quantity**, and **sex**. Source names are arbitrary. After pasting in **Input**, the GUI checks the first nonempty record: any numeric cell means headerless, otherwise headered. **Input includes header row** updates automatically and remains manually editable. Quoted/trimmed numeric cells count; embedded digits in names do not. Select header names or **Column N** entries in **Config → Input columns**, rather than typing numbers; headerless input retains its first data row. Existing named mappings must be explicitly remapped, not guessed. IDs/sex are trimmed, required, and cage IDs must be unique after trimming. Inventory order determines assignment order; cages are never sorted automatically.

The default output columns are `Cage`, `Sample`, `Ear`, `Plate`, `Well`, `#`. `Sample` is the within-cage index, starting at 1 for each cage. `#` is the global mouse number. There is exactly one output row per mouse, with no extra sex-only rows.

All mappings, assignment/ear options, and output names/order live in this workflow's **Config** pop-up, with bold **Input columns** / **Output format** titles and **Plate layout**, **Ear labels**, **Output columns** subsections. **Apply / Cancel** commits/discards its draft; **Reset** stages tool built-ins. **Save as default** saves and applies only Cage to Plate, preserving the saved Plate to Cage section. See [configuration](configuration.md).

## Assignment

Defaults: 8 lettered rows × 12 numbered columns, first plate/mouse number 1, and first well `1A`. Column-wise order is `1A, 1B, …, 1H, 2A, …, 12H`. Mouse 97 uses plate 2, well `1A`. Cages may span plates; reserved/control wells are not supported in version 1.

Settings allow row/column counts, column-wise or row-wise traversal, `1A` or `A1` spelling, first well, and starting plate/global numbers. The configured starting well only offsets the **first** plate; later plates restart at their first well. Well spelling does not change traversal. Rows beyond Z use AA, AB, … . JSON start-row/start-column indices are zero-based; the GUI displays them one-based (A=1).

Default ears are generated for the maximum cage quantity, by increasing total label length and then decreasing R count:

```text
R, L,
RR, RL, LL,
RRR, RRL, RLL, LLL,
RRRR, RRRL, RRLL, RLLL, LLLL, …
```

`LR` is equivalent to `RL` and is omitted. An empty configured ear list selects automatic generation. To customize, use **Customize inferred order**, then edit/reorder one label per line. Explicit lists must contain enough trimmed, unique, nonempty labels; labels are not recycled. Every cage restarts at the first ear label.

## Cage/sex display

Choose **Cage sex layout**:

- **Sex on a new line** (`newline`): first mouse's Cage cell is the cage ID, second mouse's Cage cell is sex, and remaining cells are blank. Sex is on another **mouse row**, not inside a multiline cell.
- **Inline sex** (`inline`): first mouse's Cage cell is `Cage (Sex)` and remaining cells are blank.
- **Separate column** (`separate_column`): repeat cage ID and sex on every mouse row. Add a `SEX` column immediately after `Cage`, including for singleton cages. This matches Plate to Cage's Separate column input layout.
- In the first two layouts, singleton cages always use inline fallback. No synthetic mouse rows are created.

Output renaming/reordering uses stable keys `cage`, `sample`, `ear`, `plate`, `well`, `mouse_number`, plus `sex` only for Separate column. **Output columns** labels their meanings (Cage ID, Within-cage ID, Ear label, Plate ID, Well ID, Global ID), while the six default names stay unchanged. Reordering never changes ear or well assignment. **Output** has copy buttons with/without headers; both preserve all body rows.

## Core functions

| API in `src/cage_to_plate.rs` | Input → output / errors |
| --- | --- |
| `read_cages(table, config)` | Resolves source mappings into normalized `Vec<Cage>`; reports source-cell errors for counts/identities/sex and ambiguous/missing columns. |
| `default_ears(count)` | Generates exactly the required prefix of the confirmed sequence; rejects count above the mouse limit. Zero returns an empty list. |
| `expand_cages(cages, config)` | Expands normalized cages into assigned `Vec<Mouse>`; validates settings, identities, quantities, ear capacity, size and numbering overflow before allocation. Generated records have source line 0 and no genotype/remark data. |
| `convert(table, config)` | Full mapped inventory → presentation-configured `OutputTable`; calls the stages above and renders the selected cage/sex layout. |
| `PlateConfig::validate()` / `CageToPlateConfig::validate()` | Input-independent validation; ear capacity is checked only when counts are known. |

All functions are deterministic and have no clipboard/file access. Limits: 100,000 mice, 2,000,000 table cells, and each plate dimension in 1..=1024. Checked arithmetic rejects plate/global numbering overflow instead of wrapping.

## Verification

`tests/example_fixtures.rs` compares both logical cells and serialized TSV against `example/cage2plate.output.txt`: 19 mice and final well `3C`. Headerless variants derived from the same fixture must produce identical cells/TSV. Unit/edge tests cover paste inference, first-row retention, numeric bounds, physical-line diagnostics, separate-sex round trips and body-only copying as well as extended/custom ears, singleton mixtures, 96/97 boundaries, non-first starts, alternate traversal/spelling, invalid counts/IDs and overflow. Run `cargo test --no-default-features`.
