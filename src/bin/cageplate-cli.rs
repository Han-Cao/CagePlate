//! Explicit, deterministic file/stdin/stdout adapter; never loads GUI defaults.

use cageplate::{
    Error, Result, cage_to_plate,
    config::AppConfig,
    plate_to_cage,
    tsv::{parse_tsv_with_header, write_tsv},
};
use std::{
    env, fs,
    io::{self, Read, Write},
    process::ExitCode,
};

const USAGE: &str = "CagePlate headless table converter\n\nUsage:\n  cageplate-cli cage-to-plate [--input PATH|-] [--output PATH|-] [--config JSON]\n  cageplate-cli plate-to-cage [--input PATH|-] [--output PATH|-] [--config JSON]\n\nInput/output default to stdin/stdout. Configuration defaults are built-in, not GUI-saved.\nFor headerless cage-to-plate input, set has_header: false and positional mappings in its config. Plate-to-cage requires headers.\nUse --help to show this message. See docs/development.md for examples.\n";
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

struct Options {
    command: String,
    input: String,
    output: String,
    config: Option<String>,
}

fn options() -> Result<Option<Options>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.iter().any(|v| v == "--help" || v == "-h") {
        return Ok(None);
    }
    let command = args
        .first()
        .filter(|c| ["cage-to-plate", "plate-to-cage"].contains(&c.as_str()))
        .ok_or_else(|| Error::new("Choose cage-to-plate or plate-to-cage; use --help for usage"))?
        .clone();
    let mut options = Options {
        command,
        input: "-".into(),
        output: "-".into(),
        config: None,
    };
    let mut seen = std::collections::HashSet::new();
    let mut index = 1;
    while index < args.len() {
        let flag = args[index].as_str();
        if !["--input", "--output", "--config"].contains(&flag) {
            return Err(Error::new(format!("Unknown argument {flag:?}; use --help")));
        }
        if !seen.insert(flag) {
            return Err(Error::new(format!("Duplicate argument {flag}")));
        }
        let value = args
            .get(index + 1)
            .filter(|v| !v.starts_with("--"))
            .ok_or_else(|| Error::new(format!("Missing value after {flag}")))?
            .clone();
        match flag {
            "--input" => options.input = value,
            "--output" => options.output = value,
            "--config" => options.config = Some(value),
            _ => unreachable!(),
        }
        index += 2;
    }
    Ok(Some(options))
}

fn read_limited(reader: impl Read, limit: u64) -> Result<String> {
    let mut text = String::new();
    reader.take(limit + 1).read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err(Error::new(format!(
            "Input exceeds the {limit}-byte safety limit"
        )));
    }
    Ok(text)
}

fn run(options: Options) -> Result<()> {
    let config = match options.config {
        Some(path) => {
            AppConfig::from_json(&read_limited(fs::File::open(path)?, MAX_CONFIG_BYTES)?)?
        }
        None => AppConfig::default(),
    };
    let input = if options.input == "-" {
        read_limited(io::stdin().lock(), MAX_INPUT_BYTES)?
    } else {
        read_limited(fs::File::open(&options.input)?, MAX_INPUT_BYTES)?
    };
    let has_header = match options.command.as_str() {
        "cage-to-plate" => config.cage_to_plate.has_header,
        "plate-to-cage" => config.plate_to_cage.has_header,
        _ => unreachable!("validated command"),
    };
    let table = parse_tsv_with_header(&input, has_header)?;
    let output = match options.command.as_str() {
        "cage-to-plate" => cage_to_plate::convert(&table, &config.cage_to_plate)?,
        "plate-to-cage" => plate_to_cage::convert(&table, &config.plate_to_cage)?,
        _ => unreachable!("validated command"),
    };
    let text = write_tsv(&output)?;
    if options.output == "-" {
        let mut stdout = io::stdout().lock();
        stdout.write_all(text.as_bytes())?;
        stdout.flush()?;
    } else {
        fs::write(&options.output, text)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match options().and_then(|options| match options {
        Some(options) => run(options),
        None => {
            print!("{USAGE}");
            Ok(())
        }
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("CagePlate: {error}");
            ExitCode::FAILURE
        }
    }
}
