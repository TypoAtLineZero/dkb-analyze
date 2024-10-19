use anyhow::{Result};
use clap::Parser;
use log::info;
use polars::prelude::*;


pub mod analyzing;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, required = true)]
    path: std::path::PathBuf,

    #[arg(short, long, required = false)]
    interval: Option<String>,

    #[arg(short, long, required = false)]
    visualization: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Starting DKB Analyze");
    
    let args = Cli::parse();
    // info!("Path: {:?}", args.path);
    // info!("Intervall: {:?}", args.interval);
    // info!("Visualization: {:?}", args.visualization);

    let df_csv = CsvReadOptions::default()
        .with_infer_schema_length(None)
        .with_has_header(true)
        .with_parse_options(CsvParseOptions::default().with_try_parse_dates(true))
        .try_into_reader_with_file_path(Some(args.path.into()))?
        .finish()?;

    println!("{}", df_csv);

    Ok(())
}
