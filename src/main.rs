use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use csv::ReaderBuilder;

mod categories;

use crate::categories::Expense;

fn main() -> Result<(), Box<dyn Error>> {
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
    println!("Category Totals:");
    for (category, total) in &category_totals {
        println!("{}: {:.2}", category, total);
    }

    Ok(())
}
