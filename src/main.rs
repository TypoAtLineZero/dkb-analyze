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
    let normalized_value = cleaned_value.replace('.', "");      // Removing thousand separator
    let normalized_value = normalized_value.replace(',', ".");  // Decimal comma to decimal point

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
    let mut uncategorized_totals: HashMap<&str, f64> = HashMap::new();
    let mut uncategorized_entries: Vec<(String, f64)> = Vec::new();

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
    let description_idx = 5;
    let _type_idx = 6;
    let _iban_idx = 7;
    let amount_idx = 8;

    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .from_reader(file);
    let mut entries_total: i32 = 0;
    let mut entries_uncategorized: i32 = 0;

    // Iterate through each record
    for result in rdr.records() {
        let record = result?;
        entries_total += 1;

        let recipient = record.get(recipient_idx).unwrap_or("").to_string();
        let recipient_lower = recipient.to_lowercase();
        let description = record.get(description_idx).unwrap_or("").to_string();
        let description_lower = description.to_lowercase();

        let amount: f64 = parse_amount(record.get(amount_idx).unwrap_or("0").to_string());
        if amount > 0.0 {
            *category_totals.entry("Income").or_insert(0.0) += amount;
            continue;
        }

        // Categorize the expense
        let mut matched = false;
        for (category, keywords) in &categories {
            if keywords.iter().any(|k| recipient_lower.contains(k)) {
                *category_totals.entry(category).or_insert(0.0) += amount;
                matched = true;
                break;
            }
        }

        // Optional: Handle uncategorized expenses
        if !matched {
            entries_uncategorized += 1;
            *uncategorized_totals.entry("Uncategorized").or_insert(0.0) += amount;
            uncategorized_entries.push((description_lower, amount));
        }
    }

    // Print uncategorized entries
    // for (description_lower, amount) in &uncategorized_entries {
    //     println!(
    //         "Description: {}, Amount: {:.2}",
    //         description_lower, amount
    //     );
    // }
    
    for (additional_field, amount) in uncategorized_entries {
        let additional_field_lower = additional_field.to_lowercase();
        let mut matched = false;
    
        // Try to categorize using additional field
        for (category, keywords) in &categories {
            if keywords.iter().any(|k| additional_field_lower.contains(k)) {
                *category_totals.entry(category).or_insert(0.0) += amount;
                matched = true;
                break;
            }
        }
    
        // If still uncategorized, add to the "Uncategorized" category
        if !matched {
            *category_totals.entry("Uncategorized").or_insert(0.0) += amount;
        }
    }
    

    // println!("length: {}", uncategorized_entries.len());
    // Print the totals for each category
    println!("--------------------------");
    println!("Evaluated records: {}", entries_total);
    println!("Uncategorized records: {}", entries_uncategorized);
    println!("Uncategorized value: {:.2?}", &uncategorized_totals["Uncategorized"]);
    println!("--------------------------");
    for (category, total) in &category_totals {
        println!("{}: {:.2}", category, total);
    }
    println!("==========================");
    // for (description, amount, description_lower) in &uncategorized_entries {
    //     println!("d: {} | a: {:.2} | d: {}", description, amount, description_lower);
    // }

    // What is missing
    // - Abstract current working directory
    // - Analyze number of uncategorized entries
    // - if entry is not found in description column, search in other columns but only with uncategorized entries

    Ok(())
}
