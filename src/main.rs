// use anyhow::Result;
// // use categories::Categories;
// use clap::Parser;
// use log::info;
// use polars::prelude::*;
// use std::path::PathBuf;
// use csv::ReaderBuilder;
// use regex::Regex;
// use std::collections::HashMap;
// use std::error::Error;
// use std::fs::File;

// pub mod categories;

// use serde::{Deserialize, Serialize};

// #[derive(Debug, Deserialize, Serialize)]
// struct Spending {
//     description: String,
//     amount: f64,
//     category: Option<String>, // Category will be set later
// }


// // #[derive(Parser, Debug)]
// // #[command(version, about, long_about = None)]
// // struct Cli {
// //     #[arg(short, long, required = true)]
// //     path: Option<PathBuf>,

// //     #[arg(short, long, required = false)]
// //     interval: Option<String>,

// //     #[arg(short, long, required = false)]
// //     visualization: bool,
// // }


// fn read_and_categorize_spendings(file_path: &str) -> Result<Vec<Spending>, Box<dyn Error>> {
//     let mut rdr = ReaderBuilder::new().has_headers(true).from_path(file_path)?;

//     let mut spendings = Vec::new();

//     for result in rdr.deserialize() {
//         let mut spending: Spending = result?;
        
//         // Categorize each spending
//         spending.category = Some(categorize_spending(&spending.description));
        
//         spendings.push(spending);
//     }

//     Ok(spendings)
// }


// fn categorize_spending(description: &str) -> String {
//     let categories = vec![
//         ("food", "Food"),
//         ("transport", "Transport"),
//         ("entertainment", "Entertainment"),
//         ("shopping", "Shopping"),
//         ("utilities", "Utilities"),
//     ];

//     // Look for a keyword in the description
//     for (keyword, category) in categories {
//         if description.to_lowercase().contains(keyword) {
//             return category.to_string();
//         }
//     }

//     // If no keyword matches, categorize as "unknown"
//     "Unknown".to_string()
// }

// fn load_csv_from_path(file_path: Option<PathBuf>) -> Result<DataFrame, PolarsError> {
//     let df = CsvReadOptions::default()
//         .try_into_reader_with_file_path(file_path)?
//         // .infer_schema(None)
//         // .with_has_header(true)
//         // .with_infer_schema_length(None)
//         // .with_has_header(true)
//         // .with_parse_options(CsvParseOptions::default().with_try_parse_dates(true))
//         .finish()?;

//     Ok(df)
// }

// fn group_by_payee(df: DataFrame) -> PolarsResult<DataFrame> {
//     df.group_by(["Zahlungsempfänger*in"])?
//         .select(["Buchungsdatum"])
//         .count()
// }

// fn sort_with_specific_order(df: PolarsResult<DataFrame>, descending: bool) -> PolarsResult<DataFrame> {
//     df?.sort(
//         ["Buchungsdatum_count"],
//         SortMultipleOptions::new().with_order_descending(descending),
//     )
// }

// fn main() -> Result<(), ::std::io::Error> {
//     env_logger::init();
//     info!("Starting DKB Analyze");

//     let file_path = "/home/franz/Downloads/csv_test_trimmed.csv"; // Replace with your CSV file path
    
//     let spendings = read_and_categorize_spendings(file_path)?;

//     // Print categorized spendings
//     for spending in spendings {
//         println!("{:?} - Category: {}", spending, spending.category.unwrap());
//     }

//     // let args = Cli::parse();
//     // // info!("Path: {:?}", args.path);
//     // // info!("Intervall: {:?}", args.interval);
//     // // info!("Visualization: {:?}", args.visualization);
//     // let df = load_csv_from_path(args.path).unwrap();
//     // println!("{}", df);
//     // // println!("{:?}", df.get_column_names());
//     // // println!("{:?}", df.get_columns());

//     // let count_df = group_by_payee(df.clone());
//     // // println!("{:?}", count_df);

//     // let _sorted_count_df = sort_with_specific_order(count_df, true);
//     // // println!("{:?}", sorted_count_df);

//     // // let mut expanses = categories::init();
//     // // println!("{:?}", expanses);
//     // // expanses.car.insurance = 5.0;
//     // // println!("{:?}", expanses);

//     // categories::get_keywords();


//     // let mut reader = Reader::from_path("/home/franz/Downloads/csv_test_trimmed.csv").unwrap();
//     // let keyword_regex = Regex::new(r"^(?i)(\bkeyword1\b|\bkeyword2\b|\bkeyword3\b)").unwrap();

//     // // Create a regular expression to match keywords against
//     // let category_map = HashMap::<String, Vec<String>>::new();
//     // category_map.insert("keyword1".to_string(), vec!["cat1".to_string()]);
//     // category_map.insert("keyword2".to_string(), vec!["cat2".to_string()]);
//     // category_map.insert("keyword3".to_string(), vec!["cat3".to_string()]);

//     //    // Iterate over the rows in the CSV file and categorize each entry
//     //    for row in reader.records() {
//     //     // Get the current entry
//     //     let entry = row[0].to_string();
        
//     //     // Check if any keyword is present in the entry using a regular expression
//     //     match keyword_regex.is_match(&entry) {
//     //         true => println!("{}", entry),
//     //         false => println!("unknown"),
//     //     }
//     // }

//         // // Read in the CSV file using the csv crate
//         // let mut reader = Reader::from_path("entries.csv").unwrap();
    
//         // // Create a regular expression to match keywords against
//         // let keyword_regex = Regex::new(r"^(?i)(\bkeyword1\b|\bkeyword2\b|\bkeyword3\b)")).unwrap();

    
//         // // Define the categories for each keyword
//         // let category_map = HashMap::<String, Vec<String>>::new();
//         // category_map.insert("keyword1".to_string(), vec!["cat1".to_string()]);
//         // category_map.insert("keyword2".to_string(), vec!["cat2".to_string()]);
//         // category_map.insert("keyword3".to_string(), vec!["cat3".to_string()]);
    
//         // // Iterate over the rows in the CSV file and categorize each entry
//         // for row in reader.records() {
//         //     // Get the current entry
//         //     let entry = row[0].to_string();
    
//         //     // Check if any keyword is present in the entry using a regular expression
//         //     match keyword_regex.is_match(&entry) {
//         //         true => println!("{}", entry),
//         //         false => println!("unknown"),
//         //     }
//         // }
    



//     Ok(())
// }

use csv::{ReaderBuilder};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;

#[derive(Debug, Deserialize, Serialize)]
struct Spending {
    description: String,
    amount: f64,
    category: String, // Changed to String, as we don't need Option anymore
}

// Function to categorize spending based on keywords
fn categorize_spending(description: &str) -> String {
    let categories = vec![
        ("food", "Food"),
        ("transport", "Transport"),
        ("entertainment", "Entertainment"),
        ("shopping", "Shopping"),
        ("utilities", "Utilities"),
    ];

    // Look for a keyword in the description
    for (keyword, category) in categories {
        if description.to_lowercase().contains(keyword) {
            return category.to_string();
        }
    }

    // If no keyword matches, categorize as "unknown"
    "Unknown".to_string()
}

// Read CSV file and categorize spendings
fn read_and_categorize_spendings(file_path: &str) -> Result<Vec<Spending>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
    
    let mut spendings = Vec::new();

    // Process each record in the CSV
    for result in rdr.deserialize() {
        let mut spending: Spending = result?;

        // Categorize each spending
        spending.category = categorize_spending(&spending.description); // Direct assignment, no Option

        spendings.push(spending);
    }

    Ok(spendings)
}

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "/home/franz/_devenv/private/dkb-analyze/src/spendings.csv"; // Replace with your CSV file path
    
    // Read and categorize spendings
    let spendings = read_and_categorize_spendings(file_path)?;

    // Print categorized spendings
    for spending in spendings {
        println!("{:?} - Category: {}", spending, spending.category);
    }

    Ok(())
}

