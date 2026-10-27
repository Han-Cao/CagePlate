# Plate → Cage

## Input normalization

Map cage display, ear, positive global mouse number, and one or more genotype columns. Optionally map a separate sex column, within-cage index, plate, well, and remarks. Unmap optional fields that are absent. A header row is required to identify genotype group names; headerless input/configs are rejected. There is no header checkbox for this workflow. Source names are arbitrary and mapping identity includes duplicate-header occurrence when needed. Missing/incompatible saved mappings require explicit remapping.

Config contains the layout, all mappings, gene hierarchy/tokens/labels, mouse ID, sex markers and output names/order. **Apply / Cancel** commits/discards this workflow's draft; **Reset** stages tool defaults. **Save as default** saves and applies only Plate to Cage, preserving the saved Cage to Plate section. It does not persist the selected genotype input columns or generated genotype output columns; instead, each selected genotype's display settings are merged into a normalized source-name profile for reuse on a future matching input. In **Input columns**, selectors are ordered Cage ID, Within-cage ID, Ear label, Plate ID, Well ID, Global ID, Remarks, Sex, followed by **Genotype (level 1)**. Optional mappings can be **Not used**. Select real header names after pasting; **Add genotype** adds another level selector below the final genotype row. Reorder/remove input rows to change levels; their settings travel with the selected genes. See [configuration](configuration.md).

Choose **Input cage sex layout** explicitly (same display names as Cage to Plate):

- **Sex on a new line** (`sparse`): cage ID on the first mouse row, a recognized sex marker on the second, and blanks thereafter. Inline `Cage (Sex)` blocks are also accepted, including cage-to-plate singleton fallbacks. Default sparse sex markers are F/M and are configurable.
- **Inline sex** (`inline`): first row of each block is `Cage (Sex)` followed by blank continuation cage cells. Parsing uses the final ` (Sex)` suffix, so cage IDs may themselves contain parentheses.
- **Separate column** (`repeated`): cage ID and a separate sex value are supplied on every row. Required for ordinary repeated cage/sex tables.

In sparse mode a sex marker is recognized only on the second mouse row of an active block. A supplied within-cage index must be 1 at a new block and 2 at its sex marker. An orphan blank row/sex marker, absent sex, or ambiguous block is an error; switch layout or correct source data rather than relying on guesses. If a cage ID itself equals a sex marker, use indices or an unambiguous inline/repeated layout.

Normalize sparse display before grouping. Repeated/disjoint blocks of a cage consolidate into one summary row. Cage rows follow first-seen order, and mice within each summary cell follow source order. Global numbers must be positive and unique across the table; supplied within-cage indices must be positive and unique within each cage. Conflicting cage sex or blank cage/sex/ear is an error.

## Genotypes and hierarchy

Select gene columns in **outermost → innermost** order in **Input columns**, using **Add genotype** below the final row for further levels and row arrows to reorder. **Genotypes** automatically displays a bold-titled card per selected source. Each card contains only **Include genotypes** and **Display names** (group/category labels and category ordering). There is no separate Add genotype column control in this section. **Accepted genotype values** configures the shared tokens below the per-genotype display cards.

Each selected gene has an independent category policy:

- **Existing** (default): include known states occurring anywhere in that gene's input column.
- **All:** include all three known states, including absent ones.

The default category order is Hom → Het → WT; users can reorder it per gene. Category sets are inferred globally, not separately per cage or parent branch. Their Cartesian product defines output leaf columns, so an unobserved combination can still have an empty cell/column. Existing does not mean “only observed multi-gene tuples.”

The configuration defines exactly four distinct nonempty source tokens, initially:

| Semantic state | Accepted token |
| --- | --- |
| Hom | `-/-` |
| Het | `+/-` |
| WT | `+/+` |
| Unknown | `?` |

Trim structural whitespace; matching is case-sensitive. **Blank always means unknown**, regardless of the configured unknown token. Any other token is an error with source row/column/value. Every selected call is validated, including other calls in a row already containing unknown; typos cannot be hidden in the unknown bucket.

A mouse with any unknown selected call belongs **only** to the shared `?` column. That column is always retained, even when empty; unknown is not multiplied into each gene's categories. Existing observations include known calls from mice whose *other* gene is unknown. If an Existing group has no known calls, there are no known-category leaf columns; metadata/unknown/remarks remain. QTY always counts every mouse exactly once.

## Output display

There is one header row per selected gene, with outer category labels repeated over every corresponding leaf. No merged cells are emitted. Fixed Cage/QTY/SEX and `?`/Remarks labels appear on the final header row; their higher-level cells are blank.

Default gene/category labels are `Group, genotype` (for example `LSL, -/-`). An unnamed positional group uses `Column N` (for example `Column 7, -/-`); set a group display name to recover a biological gene label. A group display name and each known category's header can be overridden without changing grouping identity. To match the supplied example:

1. Outer `5XFAD`: **Existing**, resulting in Het and WT groups.
2. Inner `LSL`: **All**, resulting in Hom, Het and WT columns under each outer group.
3. Override `LSL` WT to `WT`; leave the `5XFAD` WT default `5XFAD, +/+`.
4. Map `Remarks`.

Mouse IDs default to `{ear_label}{global_id}` (such as `RR8`), joined with `, `. The validated template supports only `{cage_id}`, `{within_cage_id}`, `{ear_label}`, `{global_id}`, and `{sex}`, plus literal text. `{within_cage_id}` must be mapped when used; the other fields correspond to the required input columns or normalized sex value. Braces have no escape/expression syntax. Remarks retain source text and aggregate as `ear: text`, separated with `; `; whitespace-only remarks are omitted.

**Output** offers **Copy output (with headers)** and **Copy output (without headers)**. The latter omits every genotype header level while retaining every summary row and TSV quoting/trailing empties. Both buttons are disabled for stale output.

Metadata keys are `cage`, `quantity`, `sex`, `unknown`, `remarks`. Generated leaf keys encode source-column/genotype pairs and do not depend on display names or hierarchy order. Output column moves retain every header level attached to the data. Prefer GUI controls for leaf-key reordering; the JSON is intentionally explicit rather than inferred from nonunique display labels.

## Core functions

| API in `src/plate_to_cage.rs` | Input → output / errors |
| --- | --- |
| `normalize_mouse_rows(table, config)` | Mapped TSV → normalized `Vec<Mouse>`; validates layout, every call, identity and sex consistency. |
| `summarize_cages(mice, config)` | Normalized mice → grouped `OutputTable`; validates records, bounded category combinations, templates, and presentation. |
| `convert(table, config)` | Full normalization + summary pipeline shared by GUI/CLI. |
| `format_mouse_label(mouse, template)` | Whitelisted template → label; rejects invalid syntax/fields or absent optional values. |
| `GenotypeTokens::{validate, parse, token}` | Validate four tokens; interpret one source call; obtain a semantic state's configured spelling. Validate token configuration before calling `parse` directly. |
| `GroupConfig::{validate, label}` | Validate a complete known-state ordering/labels; obtain a category header override/default. |
| `PlateToCageConfig::validate()` | Input-independent settings validation. Empty gene selection is allowed for saving defaults but not conversion. |

Limits: 1..=8 selected genes, 100,000 mice, 4,096 total output columns (4,091 genotype leaves), and 2,000,000 total table cells. Excess products fail explicitly; use Existing/fewer genes rather than truncation.

## Verification

The supplied fixture must produce exactly five cages and 11 columns, including its two-row headers, empty inner categories and `R: test remark`. Headerless tables and `has_header: false` configs must be rejected, including through the CLI. Positional leaf identities must remain stable under display/category/hierarchy changes. Core/CLI tests also cover 1/2/3 genes, global category inference, hierarchy/column changes, custom tokens/labels/templates, unknown conservation, duplicate identities, invalid blocks, comments and oversized products.
