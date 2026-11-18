//! Grouping of uncategorized spending by merchant, with recurrence detection
//! and keyword suggestions for the categories file.
//!
//! The same merchant often appears with slightly different names
//! (`REWE Markt GmbH Filiale 4711`, `REWE MARKT GMBH Fil. 0815`), and payment
//! services such as PayPal hide the real merchant in the purpose text. The
//! [`merchant_key`] normalizes both cases so that repeated payments end up in
//! one [`MerchantGroup`].

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::fmt::Write;

use chrono::{Datelike, NaiveDate};

use crate::categories::{Categorizer, Label};
use crate::money::Amount;
use crate::transaction::Transaction;

/// Payment services whose recipient name says nothing about the merchant.
const PAYMENT_SERVICES: [(&str, &str); 5] = [
    ("paypal", "PayPal"),
    ("klarna", "Klarna"),
    ("sumup", "SumUp"),
    ("stripe", "Stripe"),
    ("adyen", "Adyen"),
];

/// Phrases in the purpose text that are followed by the actual merchant.
const MERCHANT_MARKERS: [&str; 4] = ["einkauf bei ", "zahlung an ", "purchase at ", "payment to "];

/// Words that don't identify a merchant: legal forms, filler, branch markers.
const STOP_WORDS: &[&str] = &[
    "gmbh",
    "ab",
    "oy",
    "plc",
    "mbh",
    "ag",
    "kg",
    "kgaa",
    "ohg",
    "ug",
    "se",
    "sa",
    "sarl",
    "ltd",
    "inc",
    "llc",
    "bv",
    "nv",
    "co",
    "cie",
    "et",
    "und",
    "fil",
    "filiale",
    "haftungsbeschraenkt",
    "haftungsbeschränkt",
    "ihr",
];

/// At most this many words make up a key, so trailing cities, branch names
/// or order details don't split one merchant into several groups.
const MAX_KEY_WORDS: usize = 2;

/// How regularly a merchant is paid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pattern {
    /// About once a month with a similar amount (rent, subscriptions, insurance).
    Monthly,
    /// Several times per month (supermarkets, cafés).
    Frequent,
    /// More than once, but without a clear rhythm.
    Repeated,
    /// A single payment.
    OneOff,
}

impl Pattern {
    pub fn is_recurring(self) -> bool {
        self != Pattern::OneOff
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Pattern::Monthly => "monthly",
            Pattern::Frequent => "frequent",
            Pattern::Repeated => "repeated",
            Pattern::OneOff => "one-off",
        })
    }
}

/// The normalized identity of a merchant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MerchantKey {
    /// Payment service the money went through, e.g. `PayPal`.
    pub via: Option<&'static str>,
    /// Normalized merchant name, e.g. `rewe markt`. Empty when unknown.
    pub name: String,
}

impl fmt::Display for MerchantKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match (self.via, self.name.as_str()) {
            (Some(via), "") => via.to_lowercase(),
            (Some(via), name) => format!("{}: {name}", via.to_lowercase()),
            (None, "") => "(unknown)".to_string(),
            (None, name) => name.to_string(),
        };
        f.pad(&text)
    }
}

/// Derives the merchant key of a transaction.
pub fn merchant_key(tx: &Transaction) -> MerchantKey {
    // Deliberately not `tx.counterparty()`: its fallback to the other side
    // would group bookings without a recipient under the account owner.
    let other_side = if tx.amount.is_positive() {
        &tx.payer
    } else {
        &tx.recipient
    };
    let counterparty = other_side.to_lowercase();
    let via = PAYMENT_SERVICES
        .iter()
        .find(|(needle, _)| counterparty.contains(needle))
        .map(|&(_, name)| name);

    let name = match via {
        Some(_) => merchant_from_purpose(&tx.purpose),
        None => {
            let name = normalize(&counterparty);
            if name.is_empty() {
                merchant_from_purpose(&tx.purpose)
            } else {
                name
            }
        }
    };
    MerchantKey { via, name }
}

/// Extracts the merchant from a purpose text such as
/// `1043212 PP.4711.PP . Ihr Einkauf bei Spotify AB`.
fn merchant_from_purpose(purpose: &str) -> String {
    let purpose = purpose.to_lowercase();
    let text = MERCHANT_MARKERS
        .iter()
        .find_map(|marker| purpose.find(marker).map(|i| &purpose[i + marker.len()..]))
        .unwrap_or(&purpose);
    normalize(text)
}

/// Lowercase words before the first comma, without numbers, IDs, single
/// letters and stop words, limited to [`MAX_KEY_WORDS`].
fn normalize(text: &str) -> String {
    let text = text.to_lowercase();
    let head = text.split(',').next().unwrap_or("");
    head.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() > 1)
        .filter(|word| !word.chars().any(|c| c.is_ascii_digit()))
        .filter(|word| !STOP_WORDS.contains(word))
        .take(MAX_KEY_WORDS)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Uncategorized spending with one merchant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerchantGroup {
    pub key: MerchantKey,
    /// The most common original counterparty name, for display.
    pub display_name: String,
    /// Keyword that matches every transaction of the group, ready to be
    /// added to the categories file. `None` if no common keyword exists.
    pub keyword: Option<String>,
    pub pattern: Pattern,
    /// Indices into the transaction slice, oldest first.
    pub transactions: Vec<usize>,
    /// Number of distinct calendar months with a payment.
    pub months: usize,
    pub total: Amount,
    pub first: NaiveDate,
    pub last: NaiveDate,
}

impl MerchantGroup {
    pub fn count(&self) -> usize {
        self.transactions.len()
    }

    pub fn average(&self) -> Amount {
        Amount::from_cents(self.total.cents() / self.count() as i64)
    }
}

/// Groups all uncategorized spending by merchant.
///
/// Recurring merchants come first, each part sorted by total spending
/// (largest first), so the entries that improve the categorization most are
/// at the top.
pub fn uncategorized_groups(
    transactions: &[Transaction],
    categorizer: &Categorizer,
) -> Vec<MerchantGroup> {
    let mut by_key: HashMap<MerchantKey, Vec<usize>> = HashMap::new();
    for (index, tx) in transactions.iter().enumerate() {
        if categorizer.classify(tx) == Label::Uncategorized {
            by_key.entry(merchant_key(tx)).or_default().push(index);
        }
    }

    let mut groups: Vec<MerchantGroup> = by_key
        .into_iter()
        .map(|(key, mut indices)| {
            indices.sort_by_key(|&i| transactions[i].date);
            build_group(key, indices, transactions)
        })
        .collect();
    groups.sort_by(|a, b| {
        b.pattern
            .is_recurring()
            .cmp(&a.pattern.is_recurring())
            .then_with(|| a.total.cmp(&b.total))
            .then_with(|| a.key.to_string().cmp(&b.key.to_string()))
    });
    groups
}

fn build_group(
    key: MerchantKey,
    indices: Vec<usize>,
    transactions: &[Transaction],
) -> MerchantGroup {
    let txs: Vec<&Transaction> = indices.iter().map(|&i| &transactions[i]).collect();
    let dates: Vec<NaiveDate> = txs.iter().map(|t| t.date).collect();
    let amounts: Vec<Amount> = txs.iter().map(|t| t.amount).collect();
    let months: BTreeSet<(i32, u32)> = dates.iter().map(|d| (d.year(), d.month())).collect();

    MerchantGroup {
        display_name: display_name(&key, &txs),
        keyword: common_keyword(&key, &txs),
        pattern: detect_pattern(&dates, &amounts, months.len()),
        months: months.len(),
        total: amounts.iter().copied().sum(),
        first: dates[0],
        last: dates[dates.len() - 1],
        transactions: indices,
        key,
    }
}

/// The most frequent original name (ties: the alphabetically first one).
fn display_name(key: &MerchantKey, txs: &[&Transaction]) -> String {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for tx in txs {
        *counts.entry(tx.counterparty()).or_default() += 1;
    }
    let name = counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map_or("", |(name, _)| name);
    match key.via {
        Some(via) if !key.name.is_empty() => format!("{via} · {}", title_case(&key.name)),
        _ if name.is_empty() => "(unknown)".to_string(),
        _ => name.to_string(),
    }
}

fn title_case(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// A keyword that the categorizer would match for every transaction of the
/// group: the full key name, or else its single most specific word.
fn common_keyword(key: &MerchantKey, txs: &[&Transaction]) -> Option<String> {
    let haystacks: Vec<String> = txs
        .iter()
        .map(|t| format!("{}\n{}", t.recipient, t.purpose).to_lowercase())
        .collect();
    let matches_all = |keyword: &str| haystacks.iter().all(|h| h.contains(keyword));

    let mut candidates = vec![key.name.clone()];
    let mut words: Vec<&str> = key.name.split(' ').collect();
    words.sort_by_key(|w| std::cmp::Reverse(w.chars().count()));
    candidates.extend(words.into_iter().map(str::to_owned));
    if let Some(via) = key.via {
        // Last resort: the payment service itself (e.g. "paypal").
        candidates.push(via.to_lowercase());
    }
    candidates
        .into_iter()
        .filter(|c| c.chars().count() >= 3)
        .find(|c| matches_all(c))
}

fn detect_pattern(dates: &[NaiveDate], amounts: &[Amount], months: usize) -> Pattern {
    let count = dates.len();
    if count == 1 {
        return Pattern::OneOff;
    }

    let mut gaps: Vec<i64> = dates.windows(2).map(|w| (w[1] - w[0]).num_days()).collect();
    gaps.sort_unstable();
    let median_gap = gaps[gaps.len() / 2];

    let mut cents: Vec<u64> = amounts.iter().map(|a| a.cents().unsigned_abs()).collect();
    cents.sort_unstable();
    let (smallest, largest) = (cents[0], cents[cents.len() - 1]);
    // Similar amounts: the largest is at most 50 % above the smallest.
    let stable_amount = largest * 2 <= smallest * 3;

    if count >= 3 && (25..=35).contains(&median_gap) && stable_amount {
        Pattern::Monthly
    } else if count >= 4 && count >= 2 * months {
        Pattern::Frequent
    } else {
        Pattern::Repeated
    }
}

/// Renders the groups as a TOML snippet for the categories file.
pub fn suggestions_toml(groups: &[MerchantGroup]) -> String {
    let mut out = String::new();
    out.push_str(
        "# Keywords for uncategorized spending, generated by dkb-analyze.\n\
         # Paste this into categories.toml and move each line into the right category.\n\
         # Lines are sorted by importance: recurring first, then by total amount.\n",
    );
    let (with_keyword, without): (Vec<_>, Vec<_>) =
        groups.iter().partition(|g| g.keyword.is_some());

    if with_keyword.is_empty() {
        out.push_str("# No suggestions: nothing is uncategorized.\n");
    } else {
        out.push_str("Unsorted = [\n");
        let mut seen = BTreeSet::new();
        for group in with_keyword {
            let keyword = group.keyword.as_deref().unwrap_or_default();
            let prefix = if seen.insert(keyword) {
                ""
            } else {
                "# (duplicate) "
            };
            let _ = writeln!(
                out,
                "  {prefix}{:<34} # {}, {}x, total {} · {}",
                format!("{keyword:?},"),
                group.pattern,
                group.count(),
                group.total,
                group.display_name,
            );
        }
        out.push_str("]\n");
    }
    for group in without {
        let _ = writeln!(
            out,
            "# No common keyword for {} ({}x, total {}): categorize manually.",
            group.display_name,
            group.count(),
            group.total
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(date: (i32, u32, u32), recipient: &str, purpose: &str, cents: i64) -> Transaction {
        Transaction {
            date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
            payer: "Me".into(),
            recipient: recipient.into(),
            purpose: purpose.into(),
            amount: Amount::from_cents(cents),
        }
    }

    fn key(recipient: &str, purpose: &str) -> String {
        merchant_key(&tx((2024, 1, 1), recipient, purpose, -100)).to_string()
    }

    #[test]
    fn normalizes_merchant_names() {
        assert_eq!(key("REWE Markt GmbH Filiale 4711 Berlin", ""), "rewe markt");
        assert_eq!(key("REWE MARKT GMBH Fil. 0815", ""), "rewe markt");
        assert_eq!(key("Allianz Versicherungs-AG", ""), "allianz versicherungs");
        assert_eq!(
            key("Bäckerei Müller e.K., Hauptstr. 1", ""),
            "bäckerei müller"
        );
        assert_eq!(key("", "Miete Wohnung 3.OG"), "miete wohnung");
    }

    #[test]
    fn looks_through_payment_services() {
        let paypal = "PayPal Europe S.a.r.l. et Cie S.C.A 22-24 Boulevard Royal, 2449 Luxembourg";
        assert_eq!(
            key(paypal, "1043212 PP.4711.PP . Ihr Einkauf bei Spotify AB"),
            "paypal: spotify"
        );
        assert_eq!(
            key(paypal, "Ihr Einkauf bei Spotify AB, Artikel 123"),
            "paypal: spotify"
        );
        assert_eq!(key(paypal, "12345"), "paypal");
    }

    #[test]
    fn detects_patterns() {
        let monthly = |amounts: &[i64]| {
            let txs: Vec<Transaction> = amounts
                .iter()
                .enumerate()
                .map(|(i, &c)| tx((2024, i as u32 + 1, 3), "Netflix", "", c))
                .collect();
            uncategorized_groups(&txs, &Categorizer::new(vec![]))[0].pattern
        };
        assert_eq!(monthly(&[-1399, -1399, -1599, -1599]), Pattern::Monthly);
        assert_eq!(
            monthly(&[-1000, -8000, -2500]),
            Pattern::Repeated,
            "amounts vary too much"
        );
        assert_eq!(
            monthly(&[-1399, -1399]),
            Pattern::Repeated,
            "two are not a rhythm yet"
        );
        assert_eq!(monthly(&[-1399]), Pattern::OneOff);

        let shop: Vec<Transaction> = (1..=8)
            .map(|d| tx((2024, 3, d * 3), "Edeka Center", "", -2000 - d as i64 * 731))
            .collect();
        assert_eq!(
            uncategorized_groups(&shop, &Categorizer::new(vec![]))[0].pattern,
            Pattern::Frequent
        );
    }

    #[test]
    fn groups_and_ranks_uncategorized_spending() {
        let txs = vec![
            tx((2024, 1, 1), "Hausverwaltung Sonnenhof", "Miete", -98_000),
            tx(
                (2024, 2, 1),
                "HAUSVERWALTUNG SONNENHOF GMBH",
                "Miete",
                -98_000,
            ),
            tx((2024, 3, 1), "Hausverwaltung Sonnenhof", "Miete", -98_000),
            tx((2024, 2, 9), "Möbelhaus Eiche", "Sofa", -150_000),
            tx((2024, 1, 5), "REWE Markt", "", -3_000),
            tx((2024, 1, 6), "Employer", "Gehalt", 300_000),
        ];
        let categorizer = Categorizer::from_toml_str("Groceries = [\"rewe\"]").unwrap();
        let groups = uncategorized_groups(&txs, &categorizer);

        assert_eq!(
            groups.len(),
            2,
            "categorized and income bookings are ignored"
        );
        let rent = &groups[0];
        assert_eq!(rent.key.name, "hausverwaltung sonnenhof");
        assert_eq!(rent.display_name, "Hausverwaltung Sonnenhof");
        assert_eq!(rent.pattern, Pattern::Monthly);
        assert_eq!((rent.count(), rent.months), (3, 3));
        assert_eq!(rent.total, Amount::from_cents(-294_000));
        assert_eq!(rent.average(), Amount::from_cents(-98_000));
        assert_eq!(rent.transactions, [0, 1, 2]);

        // The bigger one-off comes after the recurring payment.
        assert_eq!(groups[1].pattern, Pattern::OneOff);
        assert_eq!(groups[1].keyword.as_deref(), Some("möbelhaus eiche"));
    }

    #[test]
    fn keyword_falls_back_to_a_common_word() {
        let txs = vec![
            tx((2024, 1, 1), "DB-Vertrieb GmbH", "", -1_000),
            tx((2024, 2, 1), "DB Vertrieb GmbH", "", -1_000),
        ];
        let groups = uncategorized_groups(&txs, &Categorizer::new(vec![]));
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].keyword.as_deref(), Some("vertrieb"));
    }

    #[test]
    fn suggestions_categorize_everything() {
        let paypal = "PayPal Europe S.a.r.l. et Cie S.C.A";
        let txs = vec![
            tx((2024, 1, 1), "Hausverwaltung Sonnenhof", "Miete", -98_000),
            tx((2024, 2, 1), "Hausverwaltung Sonnenhof", "Miete", -98_000),
            tx((2024, 1, 3), paypal, "Ihr Einkauf bei Spotify AB", -1_099),
            tx((2024, 2, 3), paypal, "Ihr Einkauf bei Spotify AB", -1_099),
            tx((2024, 2, 7), paypal, "Ihr Einkauf bei Onlineshop", -2_599),
            tx((2024, 1, 9), "DB-Vertrieb GmbH", "", -4_990),
            tx((2024, 1, 9), "DB Vertrieb GmbH", "", -4_990),
        ];
        let none = Categorizer::new(vec![]);
        let snippet = suggestions_toml(&uncategorized_groups(&txs, &none));

        let suggested = Categorizer::from_toml_str(&snippet).expect("snippet is valid TOML");
        for t in &txs {
            assert_ne!(
                suggested.classify(t),
                Label::Uncategorized,
                "{t:?}\n{snippet}"
            );
        }
        assert!(snippet.contains("\"spotify\""));
    }

    #[test]
    fn suggestions_without_uncategorized() {
        let snippet = suggestions_toml(&[]);
        assert!(
            Categorizer::from_toml_str(&snippet)
                .unwrap()
                .categories()
                .is_empty()
        );
    }
}
