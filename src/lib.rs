//! Analyze account exports (CSV) from the German bank DKB.
//!
//! The pipeline is:
//!
//! 1. [`parser`] reads one or more exports into [`Transaction`]s,
//! 2. [`categories`] assigns each booking a category by keyword,
//! 3. [`analysis`] sums income, expenses and categories per period,
//! 4. [`recurring`] groups uncategorized spending by merchant and suggests keywords,
//! 5. [`report`] prints a text report, or [`tui`] shows an interactive dashboard.

pub mod analysis;
pub mod categories;
pub mod money;
pub mod parser;
pub mod recurring;
pub mod report;
pub mod transaction;
pub mod tui;

use std::path::PathBuf;

pub use analysis::{Analysis, Breakdown, Granularity, Period, analyze};
pub use categories::Categorizer;
pub use money::Amount;
pub use transaction::Transaction;

/// Errors raised while loading inputs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("malformed CSV in {}: {source}", path.display())]
    Csv { path: PathBuf, source: csv::Error },

    #[error(
        "no transactions found in {} (expected a DKB CSV export with dates like 15.10.24 in the first column)",
        path.display()
    )]
    NoTransactions { path: PathBuf },

    #[error("invalid categories file {}: {message}", path.display())]
    Categories { path: PathBuf, message: String },
}
