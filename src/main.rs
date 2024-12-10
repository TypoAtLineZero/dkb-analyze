use clap::Parser;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::env;
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
    categories.insert("Car", vec!["Aral", "JUDITH NEUMEISTER", "S DirektVersicherung AG"]);
    
    // Totals for each category
    let mut category_totals: HashMap<&str, f64> = HashMap::new();

    let path = env::current_dir()?;
    let mut constructed_path = path.join("src");
    constructed_path.set_file_name("spendings.csv");

    match file_path.as_str() {
        // "" => println!("Consider handing over your own csv, continuing with example data"),
        "" => file_path.replace_range(.., "/home/franz/_devenv/private/dkb-analyze/src/spendings.csv"),
        // "" => file_path.replace_range(.., constructed_path.to_str()),
        _ => println!("Found input file"),
    }
    println!("{}", file_path);

    // Define column indices for the CSV fields
    let _posting_idx = 0; // Assuming the second column is 'description'
    let _validation_idx = 1; // Assuming the second column is 'description'
    let _status_idx = 2; // Assuming the second column is 'description'
    let _payer_idx = 3; // Assuming the second column is 'description'
    let recipient_idx = 4; // Assuming the second column is 'description'
    let _description_idx = 5; // Assuming the second column is 'description'
    let _type_idx = 6; // Assuming the second column is 'description'
    let _iban_idx = 7; // Assuming the second column is 'description'
    let amount_idx = 8;      // Assuming the third column is 'amount'

    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().has_headers(false).from_reader(file);

    // Iterate through each record
    for result in rdr.records() {
        let record = result?;

        // Assume specific column order: description is column 0, amount is column 1
        let description = record.get(recipient_idx).unwrap_or("").to_string();
        let amount: f64 = record
            .get(amount_idx)
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);

        let description_lower = description.to_lowercase();

        // Categorize the expense
        let mut matched = false;
        for (category, keywords) in &categories {
            if keywords.iter().any(|k| description_lower.contains(k)) {
                *category_totals.entry(category).or_insert(0.0) += amount;
                matched = true;
                break;
            }
        }

        // Optional: Handle uncategorized expenses
        if !matched {
            *category_totals.entry("Uncategorized").or_insert(0.0) += amount;
        }
    }

    // Print the totals for each category
    println!("Category Totals:");
    for (category, total) in &category_totals {
        println!("{}: {:.2}", category, total);
    }

    Ok(())
}
