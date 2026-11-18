//! End-to-end tests of the `dkb-analyze` binary.

use std::path::PathBuf;
use std::process::{Command, Output};

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
}

/// Runs the binary outside the repository so a personal `categories.toml`
/// in the project root cannot influence the results.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dkb-analyze"))
        .args(args)
        .current_dir(env!("CARGO_TARGET_TMPDIR"))
        .output()
        .expect("binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn monthly_report_for_dkb_export() {
    let sample = example("examples/dkb-export-sample.csv");
    let output = run(&[sample.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr(&output));

    let report = stdout(&output);
    assert!(report.contains("MONTHLY BALANCE"));
    assert!(report.contains("01.07.2024 – 24.12.2024"));
    for month in [
        "2024-07", "2024-08", "2024-09", "2024-10", "2024-11", "2024-12",
    ] {
        assert!(report.contains(month), "missing {month}");
    }
    assert!(report.contains("16.000,00 €"), "income");
    assert!(report.contains("6.083,96 €"), "balance");
    assert!(stderr(&output).contains("built-in categories"));
}

#[test]
fn custom_categories_and_multiple_files() {
    let categories = example("categories.example.toml");
    let output = run(&[
        "--categories",
        categories.to_str().unwrap(),
        "--period",
        "yearly",
        example("examples/dkb-export-sample.csv").to_str().unwrap(),
        example("examples/spendings.csv").to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let report = stdout(&output);
    assert!(report.contains("YEARLY BALANCE"));
    assert!(report.contains("Housing"));
    assert!(report.contains("Insurance"));
    // 92 bookings in the DKB sample plus 5 in the header-less sample.
    assert!(report.contains("97"));
    assert!(
        !report.contains("TOP UNCATEGORIZED"),
        "everything is categorized"
    );
}

#[test]
fn missing_input_fails() {
    let output = run(&["does-not-exist.csv"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("cannot read does-not-exist.csv"));
}

#[test]
fn missing_categories_file_fails() {
    let sample = example("examples/spendings.csv");
    let output = run(&["-c", "nope.toml", sample.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("nope.toml"));
}

#[test]
fn non_dkb_file_fails() {
    let not_csv = example("categories.example.toml");
    let output = run(&[not_csv.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("no transactions found"));
}

#[test]
fn input_is_required() {
    let output = run(&[]);
    assert!(!output.status.success());
}

#[test]
fn suggested_categories_cover_uncategorized_spending() {
    let sample = example("examples/dkb-export-sample.csv");
    let output = run(&["--suggest-categories", sample.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr(&output));
    let snippet = stdout(&output);
    assert!(snippet.contains("\"hausverwaltung sonnenhof\""));
    assert!(snippet.contains("monthly"));

    // Used as a categories file, the snippet catches every listed merchant.
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("suggested.toml");
    std::fs::write(&path, &snippet).unwrap();
    let output = run(&["-c", path.to_str().unwrap(), sample.to_str().unwrap()]);
    assert!(output.status.success(), "{}", stderr(&output));
    let report = stdout(&output);
    assert!(report.contains("Unsorted"));
    assert!(
        !report.contains("hausverwaltung sonnenhof"),
        "no longer uncategorized"
    );
}
