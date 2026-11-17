//! Aggregation of transactions into totals per period and category.

use std::collections::HashMap;
use std::fmt;

use chrono::{Datelike, NaiveDate};

use crate::categories::{Categorizer, Label};
use crate::money::Amount;
use crate::transaction::Transaction;

/// How transactions are grouped over time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Granularity {
    Weekly,
    #[default]
    Monthly,
    Yearly,
}

impl Granularity {
    pub fn period_of(self, date: NaiveDate) -> Period {
        match self {
            Granularity::Weekly => {
                let week = date.iso_week();
                Period::Week {
                    year: week.year(),
                    week: week.week(),
                }
            }
            Granularity::Monthly => Period::Month {
                year: date.year(),
                month: date.month(),
            },
            Granularity::Yearly => Period::Year(date.year()),
        }
    }

    /// Cycles weekly → monthly → yearly → weekly.
    pub fn next(self) -> Self {
        match self {
            Granularity::Weekly => Granularity::Monthly,
            Granularity::Monthly => Granularity::Yearly,
            Granularity::Yearly => Granularity::Weekly,
        }
    }
}

impl fmt::Display for Granularity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Granularity::Weekly => "Weekly",
            Granularity::Monthly => "Monthly",
            Granularity::Yearly => "Yearly",
        })
    }
}

/// A calendar week (ISO 8601), month or year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    Week { year: i32, week: u32 },
    Month { year: i32, month: u32 },
    Year(i32),
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl Period {
    /// Short label for chart axes, e.g. `Oct 24`, `W42 24`, `2024`.
    pub fn short_label(self) -> String {
        match self {
            Period::Week { year, week } => format!("W{week:02} {:02}", year.rem_euclid(100)),
            Period::Month { year, month } => {
                format!("{} {:02}", MONTHS[month as usize - 1], year.rem_euclid(100))
            }
            Period::Year(year) => year.to_string(),
        }
    }
}

impl fmt::Display for Period {
    /// Sortable label, e.g. `2024-10`, `2024-W42`, `2024`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Period::Week { year, week } => format!("{year}-W{week:02}"),
            Period::Month { year, month } => format!("{year}-{month:02}"),
            Period::Year(year) => year.to_string(),
        };
        f.pad(&label)
    }
}

/// Spending in one category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTotal {
    pub name: String,
    /// Sum of the (negative) amounts.
    pub amount: Amount,
    pub transactions: usize,
}

/// Totals for a set of transactions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Breakdown {
    pub income: Amount,
    /// Sum of all spending (negative or zero).
    pub expenses: Amount,
    pub transactions: usize,
    pub uncategorized: usize,
    pub uncategorized_amount: Amount,
    /// Spending per category, including "Uncategorized", largest first.
    pub categories: Vec<CategoryTotal>,
}

impl Breakdown {
    pub fn balance(&self) -> Amount {
        self.income + self.expenses
    }

    fn from_transactions<'a>(
        transactions: impl IntoIterator<Item = &'a Transaction>,
        categorizer: &Categorizer,
    ) -> Self {
        let mut breakdown = Breakdown::default();
        let mut per_category: HashMap<&str, (Amount, usize)> = HashMap::new();

        for tx in transactions {
            breakdown.transactions += 1;
            let label = categorizer.classify(tx);
            if label == Label::Income {
                breakdown.income += tx.amount;
                continue;
            }
            breakdown.expenses += tx.amount;
            if label == Label::Uncategorized {
                breakdown.uncategorized += 1;
                breakdown.uncategorized_amount += tx.amount;
            }
            let entry = per_category.entry(label.name()).or_default();
            entry.0 += tx.amount;
            entry.1 += 1;
        }

        breakdown.categories = per_category
            .into_iter()
            .map(|(name, (amount, transactions))| CategoryTotal {
                name: name.to_string(),
                amount,
                transactions,
            })
            .collect();
        breakdown
            .categories
            .sort_by(|a, b| a.amount.cmp(&b.amount).then_with(|| a.name.cmp(&b.name)));
        breakdown
    }
}

/// The complete result of an analysis run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    pub granularity: Granularity,
    pub overall: Breakdown,
    /// One entry per period that has transactions, in chronological order.
    pub periods: Vec<(Period, Breakdown)>,
    /// Date range covered, `None` without transactions.
    pub range: Option<(NaiveDate, NaiveDate)>,
}

/// Aggregates transactions overall and per period.
pub fn analyze(
    transactions: &[Transaction],
    categorizer: &Categorizer,
    granularity: Granularity,
) -> Analysis {
    let mut grouped: HashMap<Period, Vec<&Transaction>> = HashMap::new();
    for tx in transactions {
        grouped
            .entry(granularity.period_of(tx.date))
            .or_default()
            .push(tx);
    }
    let mut periods: Vec<(Period, Breakdown)> = grouped
        .into_iter()
        .map(|(period, txs)| (period, Breakdown::from_transactions(txs, categorizer)))
        .collect();
    periods.sort_by_key(|(period, _)| *period);

    let range = transactions
        .iter()
        .map(|t| t.date)
        .min()
        .zip(transactions.iter().map(|t| t.date).max());

    Analysis {
        granularity,
        overall: Breakdown::from_transactions(transactions, categorizer),
        periods,
        range,
    }
}

/// Spending with one counterparty that matched no category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterpartyTotal {
    pub name: String,
    pub amount: Amount,
    pub transactions: usize,
}

/// The counterparties with the most uncategorized spending: good candidates
/// for new keywords in the categories file.
pub fn top_uncategorized(
    transactions: &[Transaction],
    categorizer: &Categorizer,
    limit: usize,
) -> Vec<CounterpartyTotal> {
    let mut totals: HashMap<&str, (Amount, usize)> = HashMap::new();
    for tx in transactions
        .iter()
        .filter(|t| categorizer.classify(t) == Label::Uncategorized)
    {
        let name = match tx.counterparty() {
            "" => "(unknown)",
            name => name,
        };
        let entry = totals.entry(name).or_default();
        entry.0 += tx.amount;
        entry.1 += 1;
    }
    let mut result: Vec<CounterpartyTotal> = totals
        .into_iter()
        .map(|(name, (amount, transactions))| CounterpartyTotal {
            name: name.to_string(),
            amount,
            transactions,
        })
        .collect();
    result.sort_by(|a, b| a.amount.cmp(&b.amount).then_with(|| a.name.cmp(&b.name)));
    result.truncate(limit);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(date: (i32, u32, u32), recipient: &str, cents: i64) -> Transaction {
        Transaction {
            date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
            payer: "Payer".into(),
            recipient: recipient.into(),
            purpose: String::new(),
            amount: Amount::from_cents(cents),
        }
    }

    fn sample() -> (Vec<Transaction>, Categorizer) {
        let txs = vec![
            tx((2024, 10, 1), "me", 250_000),
            tx((2024, 10, 5), "REWE", -5_000),
            tx((2024, 10, 9), "Mystery", -1_000),
            tx((2024, 11, 1), "me", 250_000),
            tx((2024, 11, 2), "rewe city", -7_550),
            tx((2024, 11, 3), "Mystery", -2_000),
            tx((2025, 1, 2), "Bahn", -3_000),
        ];
        let categorizer =
            Categorizer::from_toml_str("Groceries = [\"rewe\"]\nTravel = [\"bahn\"]").unwrap();
        (txs, categorizer)
    }

    #[test]
    fn monthly_balance() {
        let (txs, c) = sample();
        let analysis = analyze(&txs, &c, Granularity::Monthly);

        let labels: Vec<String> = analysis
            .periods
            .iter()
            .map(|(p, _)| p.to_string())
            .collect();
        assert_eq!(labels, ["2024-10", "2024-11", "2025-01"]);

        let (_, october) = &analysis.periods[0];
        assert_eq!(october.income, Amount::from_cents(250_000));
        assert_eq!(october.expenses, Amount::from_cents(-6_000));
        assert_eq!(october.balance(), Amount::from_cents(244_000));
        assert_eq!(october.uncategorized, 1);

        let overall = &analysis.overall;
        assert_eq!(overall.transactions, 7);
        assert_eq!(overall.balance(), Amount::from_cents(500_000 - 18_550));
        assert_eq!(overall.uncategorized_amount, Amount::from_cents(-3_000));
        let names: Vec<&str> = overall.categories.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Groceries", "Travel", "Uncategorized"]);
        assert_eq!(
            analysis.range.unwrap().1,
            NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()
        );
    }

    #[test]
    fn weekly_and_yearly() {
        let (txs, c) = sample();
        let yearly = analyze(&txs, &c, Granularity::Yearly);
        assert_eq!(yearly.periods.len(), 2);
        assert_eq!(yearly.periods[1].1.expenses, Amount::from_cents(-3_000));

        // 2025-01-02 belongs to ISO week 1 of 2025, 2024-10-01 to week 40.
        let weekly = analyze(&txs, &c, Granularity::Weekly);
        assert_eq!(
            weekly.periods[0].0,
            Period::Week {
                year: 2024,
                week: 40
            }
        );
        assert_eq!(weekly.periods.last().unwrap().0.short_label(), "W01 25");
    }

    #[test]
    fn empty_input() {
        let analysis = analyze(&[], &Categorizer::default(), Granularity::Monthly);
        assert!(analysis.periods.is_empty());
        assert_eq!(analysis.range, None);
        assert_eq!(analysis.overall.balance(), Amount::ZERO);
    }

    #[test]
    fn uncategorized_ranking() {
        let (txs, c) = sample();
        let top = top_uncategorized(&txs, &c, 10);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].name, "Mystery");
        assert_eq!(top[0].transactions, 2);
        assert_eq!(top[0].amount, Amount::from_cents(-3_000));
    }
}
