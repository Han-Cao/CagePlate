//! Deterministic cage/plate table conversion, shared by the CLI and native GUI.
//!
//! The core has no display, clipboard, or implicit configuration-file dependency.
//!
//! # Example
//!
//! ```
//! use cageplate::{cage_to_plate, config::CageToPlateConfig, tsv};
//!
//! let input = tsv::parse_tsv("Cage #\tQTY\tSEX\nA\t1\tF\n")?;
//! let output = cage_to_plate::convert(&input, &CageToPlateConfig::default())?;
//! assert_eq!(output.rows[0][0], "A (F)"); // singleton fallback
//! assert_eq!(output.rows[0][4], "1A");
//! let clipboard_text = tsv::write_tsv(&output)?;
//! assert!(clipboard_text.contains("A (F)"));
//! # Ok::<(), cageplate::Error>(())
//! ```

pub mod cage_to_plate;
pub mod config;
pub mod error;
pub mod model;
pub mod plate_to_cage;
pub mod tsv;

pub use error::{Error, Result};
