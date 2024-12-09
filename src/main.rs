use clap::Parser;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use log::{info, warn};
use csv::ReaderBuilder;
use std::path::PathBuf;

mod categories;

use crate::categories::Expense;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, required = false)]
    path: Option<PathBuf>,

    #[arg(short, long, required = false)]
    interval: Option<String>,

    #[arg(short, long, required = false)]
    visualization: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting DKB Analyze");

    let args = Cli::parse();
    // Define categories and their keywords
    let mut categories: HashMap<&str, Vec<&str>> = HashMap::new();
    categories.insert("Food", vec!["restaurant", "grocery", "cafe"]);
    categories.insert("Transportation", vec!["bus", "taxi", "fuel", "train"]);
    categories.insert("Entertainment", vec!["movie", "concert", "game"]);
    
    // Totals for each category
    let mut category_totals: HashMap<&str, f64> = HashMap::new();

    // Open the CSV file
    let file = File::open("/home/franz/_devenv/private/dkb-analyze/src/spendings.csv")?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

    // Iterate through each record
    for result in rdr.deserialize() {
        let record: Expense = result?;
        let description_lower = record.description.to_lowercase();

        // Categorize the expense
        let mut matched = false;
        for (category, keywords) in &categories {
            if keywords.iter().any(|k| description_lower.contains(k)) {
                *category_totals.entry(category).or_insert(0.0) += record.amount;
                matched = true;
                break;
            }
        }

        // Optional: Handle uncategorized expenses
        if !matched {
            *category_totals.entry("Uncategorized").or_insert(0.0) += record.amount;
        }
    }

    // Print the totals for each category
    info!("Category Totals:");
    for (category, total) in &category_totals {
        info!("{}: {:.2}", category, total);
    }

    Ok(())
}
