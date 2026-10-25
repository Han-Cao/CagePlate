# Cage → Plate

## Contract

Input is a headered cage inventory with mapped **cage ID**, **positive integer quantity**, and **sex**. Source names are arbitrary. IDs/sex are trimmed, required, and cage IDs must be unique after trimming. Inventory order determines assignment order; cages are never sorted automatically.

The default output columns are `Cage`, `Sample`, `Ear`, `Plate`, `Well`, `#`. `Sample` is the within-cage index, starting at 1 for each cage. `#` is the global mouse number. There is exactly one output row per mouse, with no extra sex-only rows.

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

- **Newline:** first mouse's Cage cell is the cage ID, second mouse's Cage cell is sex, and remaining cells are blank. Sex is on another **mouse row**, not inside a multiline cell.
- **Inline:** first mouse's Cage cell is `Cage (Sex)` and remaining cells are blank.
- **Singleton:** always inline, even in newline mode. No synthetic mouse row is created.

Output renaming/reordering uses stable keys `cage`, `sample`, `ear`, `plate`, `well`, `mouse_number`. Reordering never changes ear or well assignment.

## Core functions

| API in `src/cage_to_plate.rs` | Input → output / errors |
| --- | --- |
| `read_cages(table, config)` | Resolves source mappings into normalized `Vec<Cage>`; reports source-cell errors for counts/identities/sex and ambiguous/missing columns. |
| `default_ears(count)` | Generates exactly the required prefix of the confirmed sequence; rejects count above the mouse limit. Zero returns an empty list. |
| `expand_cages(cages, config)` | Expands normalized cages into assigned `Vec<Mouse>`; validates settings, identities, quantities, ear capacity, size and numbering overflow before allocation. Generated records have source line 0 and no genotype/remark data. |
| `convert(table, config)` | Full mapped inventory → presentation-configured `OutputTable`; calls the stages above and renders sparse/inline display. |
| `PlateConfig::validate()` / `CageToPlateConfig::validate()` | Input-independent validation; ear capacity is checked only when counts are known. |

All functions are deterministic and have no clipboard/file access. Limits: 100,000 mice, 2,000,000 table cells, and each plate dimension in 1..=1024. Checked arithmetic rejects plate/global numbering overflow instead of wrapping.

## Verification

`tests/example_fixtures.rs` compares both logical cells and serialized TSV against `example/cage2plate.output.txt`: 19 mice and final well `3C`. Unit/edge tests cover extended/custom ears, singleton mixtures, 96/97 boundaries, non-first starts, alternate traversal/spelling, invalid counts/IDs and overflow. Run `cargo test --no-default-features`.
