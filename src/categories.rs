use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Expense {
    pub date: String,
    pub time: String,
    pub account_number: String,
    pub description: String,
    pub amount: f64,
}
