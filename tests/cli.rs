use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cageplate-cli"))
}
fn normalize(text: &[u8]) -> String {
    String::from_utf8(text.to_vec())
        .unwrap()
        .replace("\r\n", "\n")
        .trim_end_matches('\n')
        .into()
}

#[test]
fn only_cage_to_plate_allows_explicit_headerless_input() {
    let temp = tempfile::tempdir().unwrap();
    for (command, input, config, expected) in [
        (
            "cage-to-plate",
            "A\t1\tF\n",
            r#"{"cage_to_plate":{"has_header":false,"cage":{"column":1},"quantity":{"column":2},"sex":{"column":3}}}"#,
            "A (F)",
        ),
        (
            "plate-to-cage",
            "A (F)\tR\t1\t+/+\n",
            r#"{"plate_to_cage":{"has_header":false,"cage":{"column":1},"ear":{"column":2},"mouse_number":{"column":3},"within_cage":null,"plate":null,"well":null,"groups":[{"column":{"column":4}}]}}"#,
            "requires a header row",
        ),
    ] {
        let path = temp.path().join("explicit.json");
        fs::write(&path, config).unwrap();
        let mut child = cli()
            .args([command, "--config"])
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        if command == "plate-to-cage" {
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
            continue;
        }
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains(expected), "{text}");
        assert_eq!(text.lines().count(), 2);
    }
}

#[test]
fn both_fixture_commands_reproduce_exact_outputs() {
    for (command, name) in [
        ("cage-to-plate", "cage2plate"),
        ("plate-to-cage", "plate2cage"),
    ] {
        let result = cli()
            .current_dir(root())
            .args([
                command,
                "--input",
                &format!("example/{name}.input.txt"),
                "--config",
                &format!("tests/fixtures/{command}.example.json"),
                "--output",
                "-",
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stderr.is_empty());
        assert_eq!(
            normalize(&result.stdout),
            normalize(&fs::read(root().join(format!("example/{name}.output.txt"))).unwrap())
        );
    }
}

#[test]
fn stdin_and_file_output_work_without_saved_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("CagePlate");
    fs::create_dir(&config_dir).unwrap();
    fs::write(config_dir.join("defaults.json"), "not valid JSON").unwrap();
    let output = temp.path().join("output.tsv");
    let mut child = cli()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("APPDATA", temp.path())
        .args(["cage-to-plate", "--output"])
        .arg(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(include_bytes!("../example/cage2plate.input.txt"))
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    assert_eq!(
        normalize(&fs::read(output).unwrap()),
        normalize(include_bytes!("../example/cage2plate.output.txt"))
    );
}

#[test]
fn errors_go_only_to_stderr_with_nonzero_status() {
    let mut child = cli()
        .args(["cage-to-plate"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Cage #\tQTY\tSEX\nA\tnope\tF\n")
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    let error = String::from_utf8(result.stderr).unwrap();
    assert!(
        error.contains("row 2") && error.contains("QTY") && error.contains("nope"),
        "{error}"
    );
}

#[test]
fn argument_and_configuration_errors_are_explicit() {
    for args in [
        vec![],
        vec!["bad-command"],
        vec!["cage-to-plate", "--wrong"],
        vec!["cage-to-plate", "--input"],
        vec!["cage-to-plate", "--input", "-", "--input", "-"],
    ] {
        let result = cli().args(args).output().unwrap();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("future.json");
    fs::write(&config, "{\"schema_version\":999}").unwrap();
    let result = cli()
        .args(["cage-to-plate", "--config"])
        .arg(config)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("schema"));
    let result = cli().arg("--help").output().unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Usage:"));
}
