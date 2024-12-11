use clap::Parser;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::env;
use csv::ReaderBuilder;

mod categories_example;
mod categories;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    // Path to the user-defined categories file (optional)
    #[arg(short, long, required = false, default_value="categories_example.rs")]
    categories: Option<String>,

    // DKB csv input file
    #[arg(short, long, required = false, default_value="")]
    input: String,

    // WIP: visualization switch
    #[arg(short, long, required = false)]
    visualization: bool,
}

fn load_categories(categories_file: Option<String>) -> HashMap<&'static str, Vec<&'static str>> {
    if let Some(path) = categories_file {
        // Check if the user-specified file exists
        if fs::metadata(&path).is_ok() {
            println!("Using user-defined categories from: {}", path);
            return categories::get_categories();
        } else {
            eprintln!("Categories file '{}' not found. Falling back to default categories.", path);
        }
    }

    // Fallback to default categories
    println!("Using default categories.");
    categories_example::get_categories()
}

fn parse_amount(value: String) -> f64 {
    // Remove surrounding quotes and trim whitespace
    let cleaned_value = value.trim_matches('"').trim();
    let normalized_value = cleaned_value.replace(',', ".");
    
    // Attempt to parse as an integer first
    if let Ok(int_value) = normalized_value.parse::<i64>() {
        return int_value as f64;
    }
    
    // If integer parsing fails, attempt to parse as a float
    if let Ok(float_value) = normalized_value.parse::<f64>() {
        return float_value;
    }
    
    // Default to 0.0 if both parsing attempts fail
    0.0
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    
    let args = Cli::parse();
    let mut file_path = args.input;
    
    let categories = load_categories(args.categories);
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

    // Define column indices for the CSV fields
    let _posting_idx = 0;
    let _validation_idx = 1;
    let _status_idx = 2;
    let _payer_idx = 3;
    let recipient_idx = 4;
    let _description_idx = 5;
    let _type_idx = 6;
    let _iban_idx = 7;
    let amount_idx = 8;
    
    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().has_headers(false).from_reader(file);
    
    // Iterate through each record
    for result in rdr.records() {
        let record = result?;
        
        // Assume specific column order: description is column 0, amount is column 1
        let description = record.get(recipient_idx).unwrap_or("").to_string();
        let amount: f64 = parse_amount(record.get(amount_idx).unwrap_or("0").to_string());

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

    // What is missing
    // - Abstract current working directory
    // - Count number of entries overall
    // - Count number of uncategorized entries
    // - Analyze number of uncategorized entries
    // - if entry is not found in description column, search in other columns but only with uncategorized entries
    // - Count entries with income

    Ok(())
}
