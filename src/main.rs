use anyhow::Result;
// use categories::Categories;
use clap::Parser;
use log::info;
use polars::prelude::*;
use std::path::PathBuf;

pub mod categories;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, required = true)]
    path: Option<PathBuf>,

    #[arg(short, long, required = false)]
    interval: Option<String>,

    #[arg(short, long, required = false)]
    visualization: bool,
}

fn load_csv_from_path(file_path: Option<PathBuf>) -> Result<DataFrame, PolarsError> {
    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(file_path)?
        // .infer_schema(None)
        // .with_has_header(true)
        // .with_infer_schema_length(None)
        // .with_has_header(true)
        // .with_parse_options(CsvParseOptions::default().with_try_parse_dates(true))
        .finish()?;

    Ok(df)
}

fn group_by_payee(df: DataFrame) -> PolarsResult<DataFrame> {
    df.group_by(["Zahlungsempfänger*in"])?
        .select(["Buchungsdatum"])
        .count()
}

fn sort_with_specific_order(df: PolarsResult<DataFrame>, descending: bool) -> PolarsResult<DataFrame> {
    df?.sort(
        ["Buchungsdatum_count"],
        SortMultipleOptions::new().with_order_descending(descending),
    )
}

fn main() -> Result<(), ::std::io::Error> {
    env_logger::init();
    info!("Starting DKB Analyze");

    let args = Cli::parse();
    // info!("Path: {:?}", args.path);
    // info!("Intervall: {:?}", args.interval);
    // info!("Visualization: {:?}", args.visualization);
    let df = load_csv_from_path(args.path).unwrap();
    println!("{}", df);
    // println!("{:?}", df.get_column_names());
    // println!("{:?}", df.get_columns());

    let count_df = group_by_payee(df.clone());
    // println!("{:?}", count_df);

    let _sorted_count_df = sort_with_specific_order(count_df, true);
    // println!("{:?}", sorted_count_df);

    // let mut expanses = categories::init();
    // println!("{:?}", expanses);
    // expanses.car.insurance = 5.0;
    // println!("{:?}", expanses);

    categories::get_keywords();

    Ok(())
}
