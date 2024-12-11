use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Expense {
    pub date: String,
    pub time: String,
    pub account_number: String,
    pub description: String,
    pub amount: f64,
}

pub fn get_categories() -> HashMap<&'static str, Vec<&'static str>> {
    let mut categories = HashMap::new();
    categories.insert("Food", vec!["restaurant", "grocery", "cafe"]);
    categories.insert("Transportation", vec!["bus", "taxi", "fuel", "train"]);
    categories.insert("Entertainment", vec!["movie", "concert", "game"]);
    categories
}
