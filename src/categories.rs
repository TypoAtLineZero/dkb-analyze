//! Keyword based categorization of bookings.
//!
//! Categories are defined in a TOML file, one key per category, in priority
//! order (the first matching category wins):
//!
//! ```toml
//! Groceries = ["rewe", "edeka", "aldi"]
//! Transportation = ["deutsche bahn", "bvg"]
//! ```
//!
//! Keywords match case-insensitively as substrings, first against the
//! recipient and, if no category matched, against the purpose text.

use std::fs;
use std::path::Path;

use crate::Error;
use crate::transaction::Transaction;

/// Label for positive bookings.
pub const INCOME: &str = "Income";
/// Label for spending that matched no category.
pub const UNCATEGORIZED: &str = "Uncategorized";

/// A named category with lowercase keywords.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub name: String,
    pub keywords: Vec<String>,
}

/// Outcome of classifying one transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Label<'a> {
    Income,
    Category(&'a str),
    Uncategorized,
}

impl<'a> Label<'a> {
    pub fn name(self) -> &'a str {
        match self {
            Label::Income => INCOME,
            Label::Category(name) => name,
            Label::Uncategorized => UNCATEGORIZED,
        }
    }
}

/// An ordered list of categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Categorizer {
    categories: Vec<Category>,
}

impl Default for Categorizer {
    /// The built-in categories, used when no categories file is available.
    fn default() -> Self {
        Self::from_toml_str(DEFAULT_CATEGORIES).expect("built-in categories are valid")
    }
}

const DEFAULT_CATEGORIES: &str = r#"
Groceries = ["rewe", "edeka", "aldi", "lidl", "netto", "penny", "kaufland", "grocery"]
Food = ["restaurant", "cafe", "lieferando", "bäckerei", "baeckerei"]
Transportation = ["deutsche bahn", "db vertrieb", "bvg", "mvg", "taxi", "uber", "fuel", "aral", "shell", "train", "bus"]
Entertainment = ["netflix", "spotify", "kino", "movie", "concert", "steam", "game"]
"#;

impl Categorizer {
    pub fn new(categories: Vec<Category>) -> Self {
        let categories = categories
            .into_iter()
            .map(|c| Category {
                name: c.name,
                keywords: c
                    .keywords
                    .iter()
                    .map(|k| k.trim().to_lowercase())
                    .filter(|k| !k.is_empty())
                    .collect(),
            })
            .collect();
        Categorizer { categories }
    }

    /// Loads categories from a TOML file.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let content = fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_toml_str(&content).map_err(|message| Error::Categories {
            path: path.to_owned(),
            message,
        })
    }

    /// Parses categories from TOML (see the module documentation).
    pub fn from_toml_str(content: &str) -> Result<Self, String> {
        let table: toml::Table = content
            .parse()
            .map_err(|e: toml::de::Error| e.to_string())?;
        let mut categories = Vec::with_capacity(table.len());
        for (name, value) in table {
            if name.eq_ignore_ascii_case(INCOME) || name.eq_ignore_ascii_case(UNCATEGORIZED) {
                return Err(format!("category name {name:?} is reserved"));
            }
            let keywords = value
                .as_array()
                .ok_or_else(|| format!("category {name:?} must be a list of keywords"))?
                .iter()
                .map(|k| {
                    k.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("category {name:?} contains a non-string keyword"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            categories.push(Category { name, keywords });
        }
        Ok(Self::new(categories))
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    /// Classifies a transaction: positive amounts are income; spending is
    /// matched against the recipient first and the purpose second.
    pub fn classify(&self, tx: &Transaction) -> Label<'_> {
        if tx.amount.is_positive() {
            return Label::Income;
        }
        [&tx.recipient, &tx.purpose]
            .into_iter()
            .find_map(|text| self.find(&text.to_lowercase()))
            .map_or(Label::Uncategorized, Label::Category)
    }

    fn find(&self, text: &str) -> Option<&str> {
        self.categories
            .iter()
            .find(|c| c.keywords.iter().any(|k| text.contains(k.as_str())))
            .map(|c| c.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Amount;
    use chrono::NaiveDate;

    fn tx(recipient: &str, purpose: &str, cents: i64) -> Transaction {
        Transaction {
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            payer: String::new(),
            recipient: recipient.into(),
            purpose: purpose.into(),
            amount: Amount::from_cents(cents),
        }
    }

    #[test]
    fn classification_order() {
        let c = Categorizer::from_toml_str(
            "Shopping = [\"PayPal\"]\nGroceries = [\"rewe\"]\nTravel = [\"Hotel\"]",
        )
        .unwrap();

        assert_eq!(c.classify(&tx("Employer", "Gehalt", 100)), Label::Income);
        assert_eq!(
            c.classify(&tx("REWE Markt", "", -100)),
            Label::Category("Groceries")
        );
        // Recipient match beats an earlier category matching only the purpose.
        assert_eq!(
            c.classify(&tx("REWE", "via paypal", -100)),
            Label::Category("Groceries")
        );
        // Falls back to the purpose text.
        assert_eq!(
            c.classify(&tx("Some Bank", "HOTEL Berlin", -100)),
            Label::Category("Travel")
        );
        // First category in file order wins.
        assert_eq!(
            c.classify(&tx("paypal rewe", "", -100)),
            Label::Category("Shopping")
        );
        assert_eq!(c.classify(&tx("Unknown", "?", -100)), Label::Uncategorized);
    }

    #[test]
    fn rejects_invalid_files() {
        assert!(Categorizer::from_toml_str("Income = [\"x\"]").is_err());
        assert!(Categorizer::from_toml_str("Food = \"rewe\"").is_err());
        assert!(Categorizer::from_toml_str("Food = [1]").is_err());
        assert!(Categorizer::from_toml_str("Food = [").is_err());
    }

    #[test]
    fn ignores_empty_keywords() {
        let c = Categorizer::from_toml_str("Food = [\"\", \"  \"]").unwrap();
        assert_eq!(c.classify(&tx("anything", "", -1)), Label::Uncategorized);
    }

    #[test]
    fn default_categories_load() {
        assert!(!Categorizer::default().categories().is_empty());
    }
}
