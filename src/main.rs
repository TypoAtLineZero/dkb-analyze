use anyhow::{Context, Result};
use clap::Parser;
// use polars_core::prelude::*;
// use polars_io::prelude::*;
// use std::fs::File;

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
    let args = Cli::parse();

    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("Could not read file '{}'", args.path.display()))?;

    for line in content.lines() {
        println!("{}", line);
    }
    // println!("Path: {:?}", args.path);
    // println!("Intervall: {:?}", args.interval);
    // println!("Visualization: {:?}", args.visualization);

    // let mut df = DataFrame::empty();

    Ok(())
}
