//! Serializable settings with named or explicitly positional source mappings.

use crate::model::Genotype;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// On-disk settings schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// A source column, addressed by header/occurrence or an explicit column number.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum ColumnRef {
    /// An unambiguous exact header name.
    Name(String),
    /// A specific zero-based occurrence of a possibly duplicate header.
    Occurrence {
        /// Exact source header.
        header: String,
        /// Zero-based occurrence, not absolute column position.
        occurrence: usize,
    },
    /// An explicit one-based source position, independent of display labels.
    Position {
        /// One-based column number (1 is the leftmost column).
        column: usize,
    },
}

impl ColumnRef {
    /// Reject blank names, impossible occurrences and out-of-range column numbers.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Position { column } if !(1..=crate::model::MAX_COLUMNS).contains(column) => {
                Err(Error::new(format!(
                    "Source column number must be in 1..={}",
                    crate::model::MAX_COLUMNS
                )))
            }
            Self::Position { .. } => Ok(()),
            _ if self.header().is_some_and(|h| h.trim().is_empty())
                || matches!(self, Self::Occurrence { occurrence, .. } if *occurrence >= crate::model::MAX_COLUMNS) =>
            {
                Err(Error::new(
                    "Source mappings need a nonempty header and a valid occurrence",
                ))
            }
            _ => Ok(()),
        }
    }

    /// Return a named source header, or `None` for an explicit position.
    pub fn header(&self) -> Option<&str> {
        match self {
            Self::Name(name) => Some(name),
            Self::Occurrence { header, .. } => Some(header),
            Self::Position { .. } => None,
        }
    }

    /// Human-readable source identity, including `Column N` for positions.
    pub fn display_name(&self) -> Cow<'_, str> {
        match self {
            Self::Position { column } => format!("Column {column}").into(),
            _ => self.header().expect("named reference").into(),
        }
    }
}

impl From<&str> for ColumnRef {
    fn from(value: &str) -> Self {
        Self::Name(value.into())
    }
}

/// Plate traversal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Traversal {
    /// Visit A..H within each numbered column.
    ColumnWise,
    /// Visit numbered columns within each lettered row.
    RowWise,
}

/// Well coordinate spelling; independent of traversal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WellStyle {
    /// Example: `1A`.
    NumberLetter,
    /// Example: `A1`.
    LetterNumber,
}

/// Cage/sex presentation in the per-mouse output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CageLayout {
    /// Sex is on the second mouse row; singletons are inline.
    Newline,
    /// First row is `Cage (Sex)`.
    Inline,
    /// Repeat cage ID and sex in separate columns on every mouse row.
    SeparateColumn,
}

/// Explicit interpretation of per-mouse cage/sex input cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputLayout {
    /// Sparse cage ID, second-row sex, blank continuation cells.
    Sparse,
    /// Sparse first-row `Cage (Sex)` labels.
    Inline,
    /// Repeated cage and separate sex columns on every row.
    Repeated,
}

/// Category inclusion policy for one selected genotype column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryMode {
    /// All three known states, including empty categories.
    All,
    /// Only known states observed in this source column.
    Existing,
}

/// User-visible labels and optional stable-key column ordering.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OutputConfig {
    /// Ordered stable keys; unspecified/new keys are appended in natural order.
    pub order: Vec<String>,
    /// Overrides for the final header row, keyed by stable column identity.
    pub labels: BTreeMap<String, String>,
}

/// Physical plate and sequential assignment settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlateConfig {
    /// Number of lettered rows.
    pub rows: usize,
    /// Number of numbered columns.
    pub columns: usize,
    /// Row-wise or column-wise traversal.
    pub traversal: Traversal,
    /// Coordinate spelling.
    pub well_style: WellStyle,
    /// Zero-based first row on the first plate.
    pub start_row: usize,
    /// Zero-based first column on the first plate.
    pub start_column: usize,
    /// Positive first plate number.
    pub start_plate: u64,
    /// Positive first global mouse number.
    pub start_mouse: u64,
}

impl Default for PlateConfig {
    fn default() -> Self {
        Self {
            rows: 8,
            columns: 12,
            traversal: Traversal::ColumnWise,
            well_style: WellStyle::NumberLetter,
            start_row: 0,
            start_column: 0,
            start_plate: 1,
            start_mouse: 1,
        }
    }
}

/// Cage inventory mappings and output settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CageToPlateConfig {
    /// Whether the first nonempty input record is a header (default true).
    pub has_header: bool,
    /// Cage ID source.
    pub cage: ColumnRef,
    /// Quantity source.
    pub quantity: ColumnRef,
    /// Sex source.
    pub sex: ColumnRef,
    /// Plate assignment.
    pub plate: PlateConfig,
    /// Empty means automatically generate enough default ears; nonempty is explicit.
    pub ears: Vec<String>,
    /// Sparse or inline cage/sex display.
    pub layout: CageLayout,
    /// Output column names/order.
    pub output: OutputConfig,
}

impl Default for CageToPlateConfig {
    fn default() -> Self {
        Self {
            has_header: true,
            cage: "Cage #".into(),
            quantity: "QTY".into(),
            sex: "SEX".into(),
            plate: PlateConfig::default(),
            ears: vec![],
            layout: CageLayout::Newline,
            output: OutputConfig::default(),
        }
    }
}

/// Exactly four distinct accepted nonempty genotype tokens; blank is always unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GenotypeTokens {
    /// Homozygous mutation token.
    pub hom: String,
    /// Heterozygous token.
    pub het: String,
    /// Wild-type token.
    pub wt: String,
    /// Explicit unknown token.
    pub unknown: String,
}

impl Default for GenotypeTokens {
    fn default() -> Self {
        Self {
            hom: "-/-".into(),
            het: "+/-".into(),
            wt: "+/+".into(),
            unknown: "?".into(),
        }
    }
}

/// A selected gene and its category/header settings. Vector order defines hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupConfig {
    /// Source genotype column.
    pub column: ColumnRef,
    /// Group display name; blank uses the source header or positional `Column N`.
    pub name: String,
    /// Include all or existing known categories.
    pub mode: CategoryMode,
    /// Permutation of Hom, Het, WT.
    pub order: Vec<Genotype>,
    /// Per-state display overrides (for example WT becomes `WT`).
    pub labels: BTreeMap<Genotype, String>,
}

impl Default for GroupConfig {
    fn default() -> Self {
        Self {
            column: "Genotype".into(),
            name: String::new(),
            mode: CategoryMode::Existing,
            order: vec![Genotype::Hom, Genotype::Het, Genotype::Wt],
            labels: BTreeMap::new(),
        }
    }
}

/// Presentation settings remembered for a genotype source name.
///
/// The source column is intentionally absent: saved profiles can be reused when
/// the same genotype header appears in a future input table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GenotypeDisplayConfig {
    /// Optional group/header display name.
    pub name: String,
    /// Include all categories or only categories observed in the source.
    pub mode: CategoryMode,
    /// Ordered known genotype categories.
    pub order: Vec<Genotype>,
    /// Per-category output header overrides.
    pub labels: BTreeMap<Genotype, String>,
}

impl Default for GenotypeDisplayConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            mode: CategoryMode::Existing,
            order: vec![Genotype::Hom, Genotype::Het, Genotype::Wt],
            labels: BTreeMap::new(),
        }
    }
}

impl From<&GroupConfig> for GenotypeDisplayConfig {
    fn from(group: &GroupConfig) -> Self {
        Self {
            name: group.name.clone(),
            mode: group.mode,
            order: group.order.clone(),
            labels: group.labels.clone(),
        }
    }
}

impl GenotypeDisplayConfig {
    /// Apply this remembered presentation to a newly selected source group.
    pub fn apply_to(&self, group: &mut GroupConfig) {
        group.name = self.name.clone();
        group.mode = self.mode;
        group.order = self.order.clone();
        group.labels = self.labels.clone();
    }

    /// Validate presentation settings independently of a source column.
    pub fn validate(&self) -> crate::Result<()> {
        GroupConfig {
            column: "Genotype".into(),
            name: self.name.clone(),
            mode: self.mode,
            order: self.order.clone(),
            labels: self.labels.clone(),
        }
        .validate()
    }
}

/// Normalize a genotype source name for remembered display-profile lookup.
///
/// Trimming, collapsing whitespace, and Unicode lower-casing make matching
/// insensitive to formatting differences while retaining non-ASCII names.
pub fn normalize_genotype_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Per-mouse mappings and genotype summary settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlateToCageConfig {
    /// Must be true: genotype sources require a header row. Retained in JSON for
    /// compatibility; false is rejected rather than silently losing gene names.
    pub has_header: bool,
    /// Cage or combined cage/sex source.
    pub cage: ColumnRef,
    /// Separate sex source, required in repeated layout.
    pub sex: Option<ColumnRef>,
    /// Ear label source.
    pub ear: ColumnRef,
    /// Global mouse number source.
    pub mouse_number: ColumnRef,
    /// Optional within-cage index source.
    pub within_cage: Option<ColumnRef>,
    /// Optional plate source.
    pub plate: Option<ColumnRef>,
    /// Optional well source.
    pub well: Option<ColumnRef>,
    /// Optional remarks source.
    pub remarks: Option<ColumnRef>,
    /// Explicit source display layout.
    pub layout: InputLayout,
    /// Recognized sex markers in sparse layout.
    pub sex_tokens: Vec<String>,
    /// Ordered selected genotype groups.
    pub groups: Vec<GroupConfig>,
    /// Remembered presentation settings keyed by normalized source genotype name.
    #[serde(default)]
    pub genotype_display_defaults: BTreeMap<String, GenotypeDisplayConfig>,
    /// Accepted genotype tokens shared by the selected columns.
    pub tokens: GenotypeTokens,
    /// Whitelisted field template; no executable expressions.
    pub mouse_label: String,
    /// Output column names/order.
    pub output: OutputConfig,
}

impl Default for PlateToCageConfig {
    fn default() -> Self {
        Self {
            has_header: true,
            cage: "Cage".into(),
            sex: None,
            ear: "Ear".into(),
            mouse_number: "#".into(),
            within_cage: Some("Sample".into()),
            plate: Some("Plate".into()),
            well: Some("Well".into()),
            remarks: None,
            layout: InputLayout::Sparse,
            sex_tokens: vec!["F".into(), "M".into()],
            groups: vec![],
            genotype_display_defaults: BTreeMap::new(),
            tokens: GenotypeTokens::default(),
            mouse_label: "{ear_label}{global_id}".into(),
            output: OutputConfig::default(),
        }
    }
}

impl PlateToCageConfig {
    /// Find a remembered profile using normalized source-name matching.
    pub fn genotype_display_profile(&self, source_name: &str) -> Option<&GenotypeDisplayConfig> {
        let normalized = normalize_genotype_name(source_name);
        self.genotype_display_defaults
            .iter()
            .find(|(key, _)| normalize_genotype_name(key) == normalized)
            .map(|(_, profile)| profile)
    }

    /// Add or replace one remembered profile under its normalized source name.
    pub fn set_genotype_display_profile(
        &mut self,
        source_name: &str,
        profile: GenotypeDisplayConfig,
    ) {
        let normalized = normalize_genotype_name(source_name);
        self.genotype_display_defaults
            .retain(|key, _| normalize_genotype_name(key) != normalized);
        if !normalized.is_empty() {
            self.genotype_display_defaults.insert(normalized, profile);
        }
    }

    /// Build the settings representation written by Plate to Cage Save as default.
    ///
    /// Source genotype mappings and generated genotype output identities are
    /// transient to the current pasted table, while display profiles persist by
    /// normalized source name and are merged with profiles already on disk.
    pub fn saved_defaults_from(&self, previous: &Self) -> Self {
        let mut saved = self.clone();
        saved.groups.clear();
        saved
            .output
            .order
            .retain(|key| !key.starts_with("genotype:"));
        saved
            .output
            .labels
            .retain(|key, _| !key.starts_with("genotype:"));
        saved.genotype_display_defaults = previous.genotype_display_defaults.clone();
        for group in &self.groups {
            if let Some(source_name) = group.column.header()
                && !normalize_genotype_name(source_name).is_empty()
            {
                saved.set_genotype_display_profile(source_name, GenotypeDisplayConfig::from(group));
            }
        }
        saved
    }
}

/// Entire persisted settings document. It intentionally cannot contain animal data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    /// Settings schema version; unknown versions are rejected.
    pub schema_version: u32,
    /// Cage expansion settings.
    pub cage_to_plate: CageToPlateConfig,
    /// Genotype summary settings.
    pub plate_to_cage: PlateToCageConfig,
}

impl AppConfig {
    /// Validate the schema and both workflows without requiring pasted data.
    /// Empty gene selection is permitted in defaults; conversion requires genes.
    pub fn validate(&self) -> crate::Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(crate::Error::new(format!(
                "Unsupported configuration schema {}; expected {SCHEMA_VERSION}",
                self.schema_version
            )));
        }
        self.cage_to_plate.validate()?;
        self.plate_to_cage.validate()
    }

    /// Parse a complete or partial settings document and validate its semantics.
    /// Omitted fields use built-ins; unknown fields or schema versions fail.
    pub fn from_json(text: &str) -> crate::Result<Self> {
        let config: Self = serde_json::from_str(text)?;
        config.validate()?;
        Ok(config)
    }
}

/// Result of loading defaults. Invalid files are preserved and reported, never
/// silently overwritten; the returned configuration is always usable.
#[derive(Debug)]
pub struct LoadedDefaults {
    /// Loaded settings or built-ins on absence/failure.
    pub config: AppConfig,
    /// Visible recovery warning, absent for a missing or valid file.
    pub warning: Option<String>,
}

/// Locate the per-user settings file; no directories are created by this call.
/// Returns an error if the operating system provides no user config directory.
pub fn default_path() -> Result<PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| Error::new("Cannot locate the user configuration directory"))?;
    Ok(base.config_dir().join("CagePlate").join("defaults.json"))
}

/// Read an explicit UTF-8 JSON config, capped at 1 MiB, and validate it.
/// This never consults another config file or writes anything.
pub fn read_config(path: &Path) -> Result<AppConfig> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        return Err(Error::new("Configuration exceeds the 1 MiB safety limit"));
    }
    AppConfig::from_json(&text)
}

/// Load saved defaults without modifying disk. A missing file is normal; corrupt,
/// future-version, inaccessible, or invalid files fall back with a warning.
pub fn load_defaults(path: &Path) -> LoadedDefaults {
    let result = match path.try_exists() {
        Ok(false) => {
            return LoadedDefaults {
                config: AppConfig::default(),
                warning: None,
            };
        }
        Ok(true) => read_config(path),
        Err(error) => Err(Error::from(error)),
    };
    match result {
        Ok(config) => LoadedDefaults {
            config,
            warning: None,
        },
        Err(error) => LoadedDefaults {
            config: AppConfig::default(),
            warning: Some(format!(
                "Could not load {}: {error}. Using built-in defaults; the original file was not changed.",
                path.display()
            )),
        },
    }
}

/// Explicitly save settings (never animal records) using a same-directory temp
/// file and atomic replacement, including on Windows. Validate before touching
/// disk; errors leave an existing settings file intact. Parent folders are only
/// created by this explicit save operation.
pub fn save_defaults(path: &Path, config: &AppConfig) -> Result<()> {
    config.validate()?;
    let text = serde_json::to_string_pretty(config)?;
    if text.len() + 1 > 1024 * 1024 {
        return Err(Error::new("Configuration exceeds the 1 MiB safety limit"));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(text.as_bytes())?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| {
        Error::new(format!(
            "Cannot replace defaults {}: {}",
            path.display(),
            e.error
        ))
    })?;
    Ok(())
}

/// Selected workflow settings to persist without saving unrelated live edits.
#[derive(Debug, Clone, Copy)]
pub enum WorkflowDefaults<'a> {
    /// Save only the cage expansion defaults.
    CageToPlate(&'a CageToPlateConfig),
    /// Save only the genotype summary defaults; selected genotype source/output
    /// identities are projected out and display profiles are merged by name.
    PlateToCage(&'a PlateToCageConfig),
}

/// Merge one workflow into the latest saved defaults and atomically replace them.
///
/// A missing file starts from tool built-ins. An unreadable, invalid or unsupported
/// existing file is never replaced: return its error so the user can recover it.
/// The other workflow always comes from disk, not unsaved application state.
/// Like [`save_defaults`], this is explicit settings-only I/O, not autosave.
pub fn save_workflow_defaults(path: &Path, workflow: WorkflowDefaults<'_>) -> Result<()> {
    let mut saved = if path.try_exists()? {
        read_config(path)?
    } else {
        AppConfig::default()
    };
    match workflow {
        WorkflowDefaults::CageToPlate(settings) => saved.cage_to_plate = settings.clone(),
        WorkflowDefaults::PlateToCage(settings) => {
            saved.plate_to_cage = settings.saved_defaults_from(&saved.plate_to_cage)
        }
    }
    save_defaults(path, &saved)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            cage_to_plate: CageToPlateConfig::default(),
            plate_to_cage: PlateToCageConfig::default(),
        }
    }
}
