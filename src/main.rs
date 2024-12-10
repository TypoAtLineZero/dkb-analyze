use clap::Parser;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use log::info;
use csv::ReaderBuilder;

mod categories;

use crate::categories::Expense;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, required = false, default_value="")]
    // file_path: Option<PathBuf>,
    input: String,

    // #[arg(short, long, required = false)]
    // interval: Option<String>,

    #[arg(short, long, required = false)]
    visualization: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting DKB Analyze");

    let args = Cli::parse();
    let mut file_path = args.input;
    // Define categories and their keywords
    let mut categories: HashMap<&str, Vec<&str>> = HashMap::new();
    categories.insert("Food", vec!["restaurant", "grocery", "cafe"]);
    categories.insert("Transportation", vec!["bus", "taxi", "fuel", "train"]);
    categories.insert("Entertainment", vec!["movie", "concert", "game"]);
    
    // Totals for each category
    let mut category_totals: HashMap<&str, f64> = HashMap::new();

    match file_path.as_str() {
        // "" => println!("Consider handing over your own csv, continuing with example data"),
        "" => file_path.replace_range(.., "/home/franz/_devenv/private/dkb-analyze/src/spendings.csv"),
        _ => println!("Found input file"),
    }
    println!("{}", file_path);

    let file = File::open(file_path)?;
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
    println!("Category Totals:");
    for (category, total) in &category_totals {
        println!("{}: {:.2}", category, total);
    }

    Ok(())
}
