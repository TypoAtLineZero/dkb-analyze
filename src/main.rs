use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, ValueEnum};

use dkb_analyze::recurring::{suggestions_toml, uncategorized_groups};
use dkb_analyze::{Categorizer, Granularity, analyze, parser, report, tui};

/// Categories file picked up from the working directory when `--categories`
/// is not given.
const DEFAULT_CATEGORIES_FILE: &str = "categories.toml";

/// Analyze DKB account exports: income, expenses and balance per period,
/// broken down by category.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// One or more DKB CSV exports.
    #[arg(required = true, value_name = "CSV")]
    inputs: Vec<PathBuf>,

    /// TOML file with category keywords
    /// [default: ./categories.toml if present, otherwise built-in categories]
    #[arg(short, long, value_name = "FILE")]
    categories: Option<PathBuf>,

    /// Group transactions by week, month or year.
    #[arg(short, long, value_enum, default_value_t = PeriodArg::Monthly)]
    period: PeriodArg,

    /// Open the interactive terminal dashboard instead of printing a report.
    #[arg(short, long, visible_alias = "visualize")]
    tui: bool,

    /// Number of uncategorized merchants listed in the report (0 hides the section).
    #[arg(long, value_name = "N", default_value_t = 10)]
    top_uncategorized: usize,

    /// Print keyword suggestions for uncategorized spending as a TOML snippet
    /// for the categories file instead of the report.
    #[arg(long, conflicts_with = "tui")]
    suggest_categories: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PeriodArg {
    Weekly,
    Monthly,
    Yearly,
}

impl From<PeriodArg> for Granularity {
    fn from(arg: PeriodArg) -> Self {
        match arg {
            PeriodArg::Weekly => Granularity::Weekly,
            PeriodArg::Monthly => Granularity::Monthly,
            PeriodArg::Yearly => Granularity::Yearly,
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let categorizer = load_categorizer(cli.categories.as_deref())?;

    let mut transactions = Vec::new();
    for path in &cli.inputs {
        let statement = parser::read_file(path)?;
        for skipped in &statement.skipped {
            eprintln!(
                "warning: {}:{}: skipped row ({})",
                path.display(),
                skipped.line,
                skipped.reason
            );
        }
        transactions.extend(statement.transactions);
    }
    transactions.sort_by_key(|tx| tx.date);

    let granularity = cli.period.into();
    if cli.tui {
        tui::run(&transactions, &categorizer, granularity).context("terminal dashboard failed")?;
    } else if cli.suggest_categories {
        print!(
            "{}",
            suggestions_toml(&uncategorized_groups(&transactions, &categorizer))
        );
    } else {
        let analysis = analyze(&transactions, &categorizer, granularity);
        let uncategorized = uncategorized_groups(&transactions, &categorizer);
        print!(
            "{}",
            report::render(&analysis, &uncategorized, cli.top_uncategorized)
        );
    }
    Ok(())
}

fn load_categorizer(explicit: Option<&Path>) -> anyhow::Result<Categorizer> {
    let path = match explicit {
        Some(path) => path,
        None if Path::new(DEFAULT_CATEGORIES_FILE).is_file() => Path::new(DEFAULT_CATEGORIES_FILE),
        None => {
            eprintln!("Using built-in categories (create {DEFAULT_CATEGORIES_FILE} to customize).");
            return Ok(Categorizer::default());
        }
    };
    eprintln!("Using categories from {}.", path.display());
    Ok(Categorizer::load(path)?)
}
