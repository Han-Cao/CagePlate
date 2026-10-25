//! Cage inventory normalization, ear generation, and sequential plate assignment.

use crate::{
    Error, Result,
    config::{CageLayout, CageToPlateConfig, PlateConfig, Traversal, WellStyle},
    model::{Cage, InputTable, MAX_MICE, Mouse, OutputTable},
    tsv::{check_size, distinct_columns, positive_number},
};
use std::collections::{HashMap, HashSet};

impl PlateConfig {
    /// Validate positive dimensions/numbering and an in-bounds first well.
    /// Dimensions are capped at 1024 each to bound generated coordinates.
    pub fn validate(&self) -> Result<()> {
        if self.rows == 0 || self.columns == 0 || self.rows > 1024 || self.columns > 1024 {
            return Err(Error::new("Plate dimensions must each be in 1..=1024"));
        }
        if self.start_row >= self.rows || self.start_column >= self.columns {
            return Err(Error::new(
                "Starting well lies outside the configured plate",
            ));
        }
        if self.start_plate == 0 || self.start_mouse == 0 {
            return Err(Error::new(
                "Starting plate and mouse numbers must be positive",
            ));
        }
        Ok(())
    }
}

impl CageToPlateConfig {
    /// Validate settings independently of input. Input-dependent ear capacity is
    /// checked by [`expand_cages`]. An empty ear list means automatic generation.
    pub fn validate(&self) -> Result<()> {
        for column in [&self.cage, &self.quantity, &self.sex] {
            column.validate()?;
        }
        self.plate.validate()?;
        self.output.validate()?;
        let mut seen = HashSet::new();
        if self.ears.len() > MAX_MICE
            || self
                .ears
                .iter()
                .any(|s| s.trim().is_empty() || s != s.trim() || !seen.insert(s))
        {
            return Err(Error::new(
                "Ear labels must be nonempty, unique, trimmed, and within the mouse-count limit",
            ));
        }
        Ok(())
    }
}

/// Generate exactly `count` ear labels by length, then decreasing R count.
///
/// Sequence: R, L, RR, RL, LL, RRR, RRL, RLL, LLL, ... . Reversed combinations
/// are equivalent and omitted. Rejects requests above the mouse safety limit.
pub fn default_ears(count: usize) -> Result<Vec<String>> {
    if count > MAX_MICE {
        return Err(Error::new(format!("Ear label count exceeds {MAX_MICE}")));
    }
    let mut labels = Vec::with_capacity(count);
    let mut length = 1;
    while labels.len() < count {
        for left in 0..=length {
            labels.push(format!("{}{}", "R".repeat(length - left), "L".repeat(left)));
            if labels.len() == count {
                break;
            }
        }
        length += 1;
    }
    Ok(labels)
}

/// Read mapped inventory rows, trimming structural fields and rejecting invalid
/// counts, blank identities/sex, duplicate cages, or ambiguous source mappings.
pub fn read_cages(table: &InputTable, config: &CageToPlateConfig) -> Result<Vec<Cage>> {
    table.validate()?;
    let cage = config.cage.resolve(table)?;
    let qty = config.quantity.resolve(table)?;
    let sex = config.sex.resolve(table)?;
    distinct_columns(&[cage, qty, sex])?;
    let mut seen = HashSet::new();
    let mut cages = Vec::with_capacity(table.rows.len());
    for (index, row) in table.rows.iter().enumerate() {
        let id = row[cage].trim();
        let sex_value = row[sex].trim();
        if id.is_empty() || !seen.insert(id) {
            return Err(Error::cell(
                table.source_rows[index],
                &table.headers[cage],
                id,
                "Cage ID must be nonempty and unique",
            ));
        }
        if sex_value.is_empty() {
            return Err(Error::cell(
                table.source_rows[index],
                &table.headers[sex],
                sex_value,
                "Sex must be specified",
            ));
        }
        cages.push(Cage {
            id: id.into(),
            quantity: positive_number(table, index, qty)?,
            sex: sex_value.into(),
        });
    }
    Ok(cages)
}

fn row_letters(mut row: usize) -> String {
    let mut chars = vec![];
    loop {
        chars.push((b'A' + (row % 26) as u8) as char);
        if row < 26 {
            break;
        }
        row = row / 26 - 1;
    }
    chars.into_iter().rev().collect()
}

fn assignment(index: usize, plate: &PlateConfig) -> Result<(u64, String)> {
    let capacity = plate.rows * plate.columns;
    let start = match plate.traversal {
        Traversal::ColumnWise => plate.start_column * plate.rows + plate.start_row,
        Traversal::RowWise => plate.start_row * plate.columns + plate.start_column,
    };
    let absolute = start
        .checked_add(index)
        .ok_or_else(|| Error::new("Well offset overflow"))?;
    let number = plate
        .start_plate
        .checked_add((absolute / capacity) as u64)
        .ok_or_else(|| Error::new("Plate number overflow"))?;
    let offset = absolute % capacity;
    let (row, column) = match plate.traversal {
        Traversal::ColumnWise => (offset % plate.rows, offset / plate.rows),
        Traversal::RowWise => (offset / plate.columns, offset % plate.columns),
    };
    let letter = row_letters(row);
    let well = match plate.well_style {
        WellStyle::NumberLetter => format!("{}{letter}", column + 1),
        WellStyle::LetterNumber => format!("{letter}{}", column + 1),
    };
    Ok((number, well))
}

/// Expand normalized cages in source order, resetting ear/within-cage numbers.
///
/// Validates positive counts/unique cages, explicit ear capacity, mouse/table
/// limits, starting settings, and numbering overflow before allocating records.
/// Generated records have `source_row=0` and no genotype/remark data.
pub fn expand_cages(cages: &[Cage], config: &CageToPlateConfig) -> Result<Vec<Mouse>> {
    config.validate()?;
    let mut seen = HashSet::new();
    let mut total = 0usize;
    let mut maximum = 0;
    if cages.is_empty() {
        return Err(Error::new("At least one cage is required"));
    }
    for cage in cages {
        if cage.id.trim().is_empty()
            || cage.sex.trim().is_empty()
            || cage.quantity == 0
            || !seen.insert(&cage.id)
        {
            return Err(Error::new(
                "Cages must have unique nonempty IDs, a sex, and a positive quantity",
            ));
        }
        total = total
            .checked_add(cage.quantity)
            .filter(|n| *n <= MAX_MICE)
            .ok_or_else(|| Error::new(format!("Expansion exceeds {MAX_MICE} mice")))?;
        maximum = maximum.max(cage.quantity);
    }
    check_size(total + 1, 6)?;
    config
        .plate
        .start_mouse
        .checked_add((total - 1) as u64)
        .ok_or_else(|| Error::new("Global mouse number overflow"))?;
    assignment(total - 1, &config.plate)?;
    let ears = if config.ears.is_empty() {
        default_ears(maximum)?
    } else {
        config.ears.clone()
    };
    if ears.len() < maximum {
        return Err(Error::new(format!(
            "Need {maximum} ear labels, but only {} are configured",
            ears.len()
        )));
    }
    let mut mice = Vec::with_capacity(total);
    for cage in cages {
        for (index, ear) in ears.iter().take(cage.quantity).enumerate() {
            let (plate, well) = assignment(mice.len(), &config.plate)?;
            mice.push(Mouse {
                cage_id: cage.id.clone(),
                sex: cage.sex.clone(),
                within_cage: Some(index + 1),
                ear: ear.clone(),
                mouse_number: config.plate.start_mouse + mice.len() as u64,
                plate: Some(plate.to_string()),
                well: Some(well),
                genotypes: vec![],
                remark: String::new(),
                source_row: 0,
            });
        }
    }
    Ok(mice)
}

/// Convert a mapped cage TSV table to the configured per-mouse output table.
///
/// Singletons always render `Cage (Sex)`. Other cages use the selected sparse or
/// inline layout. Output naming/reordering never changes assignment identities.
pub fn convert(table: &InputTable, config: &CageToPlateConfig) -> Result<OutputTable> {
    let cages = read_cages(table, config)?;
    let mice = expand_cages(&cages, config)?;
    let quantities: HashMap<_, _> = cages.iter().map(|c| (c.id.as_str(), c.quantity)).collect();
    let rows = mice
        .into_iter()
        .map(|m| {
            let index = m.within_cage.expect("generated index");
            let inline = config.layout == CageLayout::Inline || quantities[m.cage_id.as_str()] == 1;
            let cage_cell = if index == 1 {
                if inline {
                    format!("{} ({})", m.cage_id, m.sex)
                } else {
                    m.cage_id
                }
            } else if index == 2 && !inline {
                m.sex
            } else {
                String::new()
            };
            vec![
                cage_cell,
                index.to_string(),
                m.ear,
                m.plate.unwrap_or_default(),
                m.well.unwrap_or_default(),
                m.mouse_number.to_string(),
            ]
        })
        .collect();
    OutputTable {
        keys: ["cage", "sample", "ear", "plate", "well", "mouse_number"]
            .map(String::from)
            .to_vec(),
        headers: vec![
            ["Cage", "Sample", "Ear", "Plate", "Well", "#"]
                .map(String::from)
                .to_vec(),
        ],
        rows,
    }
    .present(&config.output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cage(quantity: usize) -> Cage {
        Cage {
            id: "A".into(),
            sex: "F".into(),
            quantity,
        }
    }

    #[test]
    fn extended_ears_and_custom_validation() {
        assert_eq!(
            default_ears(14).unwrap(),
            [
                "R", "L", "RR", "RL", "LL", "RRR", "RRL", "RLL", "LLL", "RRRR", "RRRL", "RRLL",
                "RLLL", "LLLL"
            ]
        );
        let mut cfg = CageToPlateConfig {
            ears: vec!["X".into()],
            ..Default::default()
        };
        assert!(expand_cages(&[cage(2)], &cfg).is_err());
        cfg.ears.push("X".into());
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn plate_boundary_keeps_every_mouse() {
        let mice = expand_cages(&[cage(97)], &CageToPlateConfig::default()).unwrap();
        assert_eq!(mice[95].well.as_deref(), Some("12H"));
        assert_eq!(mice[96].well.as_deref(), Some("1A"));
        assert_eq!(mice[96].plate.as_deref(), Some("2"));
        assert_eq!(mice[96].mouse_number, 97);
        assert_eq!(
            expand_cages(&[cage(96)], &CageToPlateConfig::default())
                .unwrap()
                .len(),
            96
        );
    }

    #[test]
    fn alternate_plate_settings_and_overflow() {
        let mut cfg = CageToPlateConfig {
            plate: PlateConfig {
                rows: 2,
                columns: 3,
                start_row: 1,
                start_column: 1,
                start_plate: 7,
                start_mouse: 41,
                ..Default::default()
            },
            ..Default::default()
        };
        let mice = expand_cages(&[cage(4)], &cfg).unwrap();
        assert_eq!(
            mice.iter()
                .map(|m| m.well.as_deref().unwrap())
                .collect::<Vec<_>>(),
            ["2B", "3A", "3B", "1A"]
        );
        assert_eq!(mice[3].plate.as_deref(), Some("8"));
        cfg.plate.traversal = Traversal::RowWise;
        cfg.plate.well_style = WellStyle::LetterNumber;
        let mice = expand_cages(&[cage(3)], &cfg).unwrap();
        assert_eq!(
            mice.iter()
                .map(|m| m.well.as_deref().unwrap())
                .collect::<Vec<_>>(),
            ["B2", "B3", "A1"]
        );
        cfg.plate.start_mouse = u64::MAX;
        assert!(expand_cages(&[cage(2)], &cfg).is_err());
        cfg.plate.start_mouse = 1;
        cfg.plate.start_plate = u64::MAX;
        assert!(expand_cages(&[cage(3)], &cfg).is_err());
    }

    #[test]
    fn singleton_is_inline_in_newline_mode() {
        let input = crate::tsv::parse_tsv("Cage #\tQTY\tSEX\nA\t1\tF\nB\t2\tM").unwrap();
        let out = convert(&input, &CageToPlateConfig::default()).unwrap();
        assert_eq!(
            out.rows.iter().map(|r| r[0].as_str()).collect::<Vec<_>>(),
            ["A (F)", "B", "M"]
        );
        assert_eq!(
            out.rows.iter().map(|r| r[1].as_str()).collect::<Vec<_>>(),
            ["1", "1", "2"]
        );
    }
}
