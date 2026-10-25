//! Headered, quoted TSV parsing, stable mappings, and output presentation.

use crate::{
    Error, Result,
    config::{ColumnRef, OutputConfig},
    model::{InputTable, MAX_CELLS, MAX_COLUMNS, MAX_MICE, OutputTable},
};
use std::collections::HashSet;

pub(crate) fn check_size(rows: usize, columns: usize) -> Result<()> {
    if columns == 0 || columns > MAX_COLUMNS {
        return Err(Error::new(format!(
            "Table must have 1..={MAX_COLUMNS} columns"
        )));
    }
    if rows.checked_mul(columns).is_none_or(|n| n > MAX_CELLS) {
        return Err(Error::new(format!(
            "Table exceeds the {MAX_CELLS}-cell safety limit; reduce input size or genotype groups"
        )));
    }
    Ok(())
}

// csv intentionally accepts some malformed quotes. Reject them before parsing so
// a missing closing quote cannot silently swallow following mouse rows.
fn validate_quotes(text: &str) -> Result<()> {
    let mut state = 0; // 0=field start, 1=unquoted, 2=quoted, 3=after quote
    let mut line = 1;
    for byte in text.bytes() {
        state = match (state, byte) {
            (0, b'"') => 2,
            (2, b'"') => 3,
            (3, b'"') => 2,
            (2, _) => 2,
            (_, b'\t' | b'\r' | b'\n') => 0,
            (3, _) | (1, b'"') => {
                return Err(Error::new(format!(
                    "Malformed TSV quoting at line {line}; quote the entire cell and double embedded quotes"
                )));
            }
            _ => 1,
        };
        if byte == b'\n' {
            line += 1;
        }
    }
    if state == 2 {
        return Err(Error::new(format!("Unclosed quoted cell at line {line}")));
    }
    Ok(())
}

/// Parse a nonempty, headered TSV table, preserving free-text cells and positions.
///
/// Supports a BOM, quoted tabs/newlines, CRLF, and trailing empty fields. Wholly
/// empty records are ignored. Returns actionable errors for malformed quoting,
/// ragged rows, empty headers/data, or documented size limits.
pub fn parse_tsv(text: &str) -> Result<InputTable> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    validate_quotes(text)?;
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut line_starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' || (byte == b'\r' && text.as_bytes().get(index + 1) != Some(&b'\n')) {
            line_starts.push(index + 1);
        }
    }
    let mut headers: Vec<String> = vec![];
    let mut rows = vec![];
    let mut source_rows = vec![];
    for record in reader.records() {
        let record = record.map_err(|e| Error::new(format!("Invalid TSV: {e}")))?;
        if record.iter().all(|s| s.trim().is_empty()) {
            continue;
        }
        // csv positions can precede the LF of a CRLF record separator.
        let offset = record.position().map_or(0, |p| p.byte() as usize);
        let start = offset
            + text.as_bytes()[offset..]
                .iter()
                .take_while(|&&b| b == b'\r' || b == b'\n')
                .count();
        let line = line_starts.partition_point(|&position| position <= start);
        if headers.is_empty() {
            headers = record.iter().map(String::from).collect();
            if headers.iter().any(|s| s.trim().is_empty()) {
                return Err(Error::new(format!(
                    "Empty source column name at line {line}; name every input column"
                )));
            }
            check_size(1, headers.len())?;
        } else {
            if record.len() != headers.len() {
                return Err(Error::new(format!(
                    "Ragged TSV row at line {line}: expected {} cells, found {}; retain trailing tabs for empty cells",
                    headers.len(),
                    record.len()
                )));
            }
            if rows.len() >= MAX_MICE {
                return Err(Error::new(format!("Input exceeds {MAX_MICE} data rows")));
            }
            check_size(rows.len() + 2, headers.len())?;
            source_rows.push(line);
            rows.push(record.iter().map(String::from).collect());
        }
    }
    if headers.is_empty() || rows.is_empty() {
        return Err(Error::new("Paste a header row and at least one data row"));
    }
    Ok(InputTable {
        headers,
        rows,
        source_rows,
    })
}

impl InputTable {
    /// Validate a programmatically constructed table before indexing its cells.
    pub fn validate(&self) -> Result<()> {
        check_size(self.rows.len() + 1, self.headers.len())?;
        if self.rows.is_empty()
            || self.rows.len() > MAX_MICE
            || self.rows.len() != self.source_rows.len()
        {
            return Err(Error::new("Invalid input row count or source positions"));
        }
        if self.headers.iter().any(|s| s.trim().is_empty())
            || self.rows.iter().any(|r| r.len() != self.headers.len())
        {
            return Err(Error::new(
                "Input requires named headers and rectangular rows",
            ));
        }
        Ok(())
    }
}

impl ColumnRef {
    /// Resolve a saved mapping against the current headers, rejecting absent or
    /// ambiguous names instead of using a stale position.
    pub fn resolve(&self, table: &InputTable) -> Result<usize> {
        let matches: Vec<_> = table
            .headers
            .iter()
            .enumerate()
            .filter(|(_, h)| h.as_str() == self.header())
            .map(|(i, _)| i)
            .collect();
        match self {
            Self::Name(_) if matches.len() == 1 => Ok(matches[0]),
            Self::Name(_) if matches.len() > 1 => Err(Error::new(format!(
                "Column {:?} is duplicated; select a specific occurrence",
                self.header()
            ))),
            Self::Occurrence { occurrence, .. } => {
                matches.get(*occurrence).copied().ok_or_else(|| {
                    Error::new(format!(
                        "Missing column {:?}, occurrence {}; remap the pasted table",
                        self.header(),
                        occurrence + 1
                    ))
                })
            }
            _ => Err(Error::new(format!(
                "Missing column {:?}; remap the pasted table",
                self.header()
            ))),
        }
    }

    /// Construct a stable mapping from a selected actual column position.
    /// Returns an error for an out-of-range selection.
    pub fn from_index(table: &InputTable, index: usize) -> Result<Self> {
        let header = table
            .headers
            .get(index)
            .ok_or_else(|| Error::new("Column index out of range"))?;
        if table.headers.iter().filter(|h| *h == header).count() == 1 {
            return Ok(Self::Name(header.clone()));
        }
        let occurrence = table.headers[..index]
            .iter()
            .filter(|h| *h == header)
            .count();
        Ok(Self::Occurrence {
            header: header.clone(),
            occurrence,
        })
    }
}

pub(crate) fn distinct_columns(indices: &[usize]) -> Result<()> {
    let mut seen = HashSet::new();
    if indices.iter().any(|i| !seen.insert(i)) {
        return Err(Error::new(
            "Different input roles must map to different source columns",
        ));
    }
    Ok(())
}

pub(crate) fn positive_number<T: std::str::FromStr + PartialEq + Default>(
    table: &InputTable,
    row: usize,
    column: usize,
) -> Result<T> {
    let text = table.rows[row][column].trim();
    text.parse::<T>()
        .ok()
        .filter(|v| *v != T::default())
        .ok_or_else(|| {
            Error::cell(
                table.source_rows[row],
                &table.headers[column],
                text,
                "Expected a positive integer",
            )
        })
}

impl OutputTable {
    /// Check rectangular shape, unique keys, and size limits before rendering.
    pub fn validate(&self) -> Result<()> {
        check_size(self.headers.len() + self.rows.len(), self.keys.len())?;
        let mut seen = HashSet::new();
        if self.headers.is_empty()
            || self.keys.iter().any(|k| !seen.insert(k))
            || self
                .headers
                .iter()
                .chain(&self.rows)
                .any(|r| r.len() != self.keys.len())
        {
            return Err(Error::new(
                "Invalid output shape or duplicate column identities",
            ));
        }
        Ok(())
    }

    /// Rename final-row headers and reorder whole columns, retaining all header
    /// levels. Stale keys are ignored; new keys are appended in natural order.
    pub fn present(mut self, config: &OutputConfig) -> Result<Self> {
        self.validate()?;
        config.validate()?;
        for (i, key) in self.keys.iter().enumerate() {
            if let Some(label) = config.labels.get(key) {
                self.headers.last_mut().expect("validated headers")[i] = label.clone();
            }
        }
        let mut indices = vec![];
        for key in &config.order {
            if let Some(index) = self.keys.iter().position(|k| k == key) {
                indices.push(index);
            }
        }
        for index in 0..self.keys.len() {
            if !indices.contains(&index) {
                indices.push(index);
            }
        }
        self.keys = indices.iter().map(|&i| self.keys[i].clone()).collect();
        for row in self.headers.iter_mut().chain(&mut self.rows) {
            *row = indices.iter().map(|&i| row[i].clone()).collect();
        }
        Ok(self)
    }
}

impl OutputConfig {
    /// Reject duplicate ordering identities or empty display-name overrides.
    pub fn validate(&self) -> Result<()> {
        let mut seen = HashSet::new();
        if self.order.iter().any(|k| !seen.insert(k)) {
            return Err(Error::new(
                "Output column order contains duplicate identities",
            ));
        }
        if self.labels.values().any(|v| v.trim().is_empty()) {
            return Err(Error::new("Output display names must not be empty"));
        }
        Ok(())
    }
}

/// Serialize all header/body rows as quoted TSV, including trailing empty cells.
/// Returns an error for malformed output shape or serialization failure.
pub fn write_tsv(table: &OutputTable) -> Result<String> {
    table.validate()?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(vec![]);
    for row in table.headers.iter().chain(&table.rows) {
        writer
            .write_record(row)
            .map_err(|e| Error::new(format!("Cannot encode TSV: {e}")))?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| Error::new(format!("Cannot finish TSV: {e}")))?;
    String::from_utf8(bytes).map_err(|e| Error::new(format!("Cannot encode text: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_bom_crlf_and_trailing_empty_cells() {
        let input =
            "\u{feff}Cage\tRemarks\tOther\r\nA\t\"  text\tline\n\"\"quoted\"\"  \"\t\r\n\t\t\r\n";
        let t = parse_tsv(input).unwrap();
        assert_eq!(t.rows[0], ["A", "  text\tline\n\"quoted\"  ", ""]);
        assert_eq!(t.source_rows, [2]);
        let out = OutputTable {
            keys: vec!["a".into(), "b".into(), "c".into()],
            headers: vec![t.headers.clone()],
            rows: t.rows.clone(),
        };
        assert_eq!(parse_tsv(&write_tsv(&out).unwrap()).unwrap().rows, t.rows);
    }

    #[test]
    fn malformed_or_empty_input_is_rejected() {
        for text in [
            "",
            "Cage\tSex\n",
            "Cage\tSex\nA",
            "Cage\tSex\nA\t\"unclosed",
            "Cage\tSex\nA\t\"quoted\"oops",
            "Cage\t\nA\tF",
        ] {
            assert!(parse_tsv(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn mappings_are_not_stale_positions() {
        let t = parse_tsv("Sex\tCage\tCage\nF\tA\tB").unwrap();
        assert!(ColumnRef::from("Cage").resolve(&t).is_err());
        assert_eq!(
            ColumnRef::from_index(&t, 2).unwrap().resolve(&t).unwrap(),
            2
        );
        assert!(ColumnRef::from("Missing").resolve(&t).is_err());
        assert!(ColumnRef::from_index(&t, 3).is_err());
        assert_eq!(ColumnRef::from("Sex").resolve(&t).unwrap(), 0);
    }

    #[test]
    fn presentation_moves_full_header_paths() {
        let t = OutputTable {
            keys: vec!["a".into(), "b".into()],
            headers: vec![
                vec!["outer a".into(), "outer b".into()],
                vec!["a".into(), "b".into()],
            ],
            rows: vec![vec!["1".into(), "2".into()]],
        };
        let cfg = OutputConfig {
            order: vec!["stale".into(), "b".into()],
            labels: [("b".into(), "renamed".into())].into(),
        };
        let t = t.present(&cfg).unwrap();
        assert_eq!(t.headers[0], ["outer b", "outer a"]);
        assert_eq!(t.headers[1], ["renamed", "a"]);
        assert_eq!(t.rows[0], ["2", "1"]);
    }
}
