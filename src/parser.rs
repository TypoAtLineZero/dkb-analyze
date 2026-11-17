//! Reading DKB CSV exports.
//!
//! Supported inputs:
//! * current DKB exports (semicolon separated, a few metadata lines before
//!   the German header row, `dd.mm.yy` dates),
//! * legacy DKB exports ("Buchungstag", "Auftraggeber / Begünstigter",
//!   ISO-8859-1 encoded),
//! * header-less or English-header files using the DKB column layout
//!   (comma or semicolon separated).
//!
//! Rows that do not start with a date (metadata, empty lines, headers) are
//! ignored. Rows that start with a date but cannot be parsed are reported in
//! [`Statement::skipped`] instead of being silently treated as zero.

use std::fs;
use std::path::Path;

use chrono::NaiveDate;
use csv::{ReaderBuilder, StringRecord};

use crate::Error;
use crate::money::Amount;
use crate::transaction::Transaction;

/// The result of parsing one export file.
#[derive(Debug, Default)]
pub struct Statement {
    pub transactions: Vec<Transaction>,
    pub skipped: Vec<SkippedRow>,
}

/// A data row that could not be turned into a [`Transaction`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRow {
    /// 1-based line number in the input.
    pub line: u64,
    pub reason: String,
}

/// Reads and parses a DKB export from disk.
pub fn read_file(path: &Path) -> Result<Statement, Error> {
    let bytes = fs::read(path).map_err(|source| Error::Io {
        path: path.to_owned(),
        source,
    })?;
    let statement = parse_str(&decode(&bytes)).map_err(|source| Error::Csv {
        path: path.to_owned(),
        source,
    })?;
    if statement.transactions.is_empty() {
        return Err(Error::NoTransactions {
            path: path.to_owned(),
        });
    }
    Ok(statement)
}

/// Parses the textual content of a DKB export.
pub fn parse_str(content: &str) -> Result<Statement, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .delimiter(sniff_delimiter(content))
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut columns = Columns::DEFAULT;
    let mut statement = Statement::default();

    for record in reader.records() {
        let record = record?;
        let line = record.position().map_or(0, |p| p.line());

        if let Some(found) = Columns::from_header(&record) {
            columns = found;
            continue;
        }
        let Some(date) = record.get(columns.date).and_then(parse_date) else {
            continue; // metadata, blank line or anything else that is not a booking
        };

        let amount = match record.get(columns.amount) {
            Some(raw) => match raw.parse::<Amount>() {
                Ok(amount) => amount,
                Err(e) => {
                    statement.skipped.push(SkippedRow {
                        line,
                        reason: e.to_string(),
                    });
                    continue;
                }
            },
            None => {
                statement.skipped.push(SkippedRow {
                    line,
                    reason: "missing amount column".into(),
                });
                continue;
            }
        };

        let field = |idx: Option<usize>| {
            idx.and_then(|i| record.get(i))
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        statement.transactions.push(Transaction {
            date,
            payer: field(columns.payer),
            recipient: field(columns.recipient),
            purpose: field(columns.purpose),
            amount,
        });
    }

    Ok(statement)
}

/// Column positions of the fields we care about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Columns {
    date: usize,
    amount: usize,
    payer: Option<usize>,
    recipient: Option<usize>,
    purpose: Option<usize>,
}

impl Columns {
    /// Layout of the current DKB export, used when no header row is present.
    const DEFAULT: Columns = Columns {
        date: 0,
        payer: Some(3),
        recipient: Some(4),
        purpose: Some(5),
        amount: 8,
    };

    /// Recognizes a header row (German or English) and maps its columns.
    fn from_header(record: &StringRecord) -> Option<Columns> {
        let names: Vec<String> = record.iter().map(|f| f.trim().to_lowercase()).collect();
        let find = |pred: &dyn Fn(&str) -> bool| names.iter().position(|n| pred(n));

        let date = find(&|n| {
            n.starts_with("buchungsdatum")
                || n.starts_with("buchungstag")
                || n.starts_with("posting date")
                || n == "date"
        })?;
        let amount = find(&|n| n.starts_with("betrag") || n.starts_with("amount"))?;
        // Legacy exports have a single counterparty column for both directions.
        let counterparty = find(&|n| n.contains("begünstigter"));

        Some(Columns {
            date,
            amount,
            payer: find(&|n| n.starts_with("zahlungspflichtige") || n == "payer").or(counterparty),
            recipient: find(&|n| n.starts_with("zahlungsempfänger") || n == "recipient")
                .or(counterparty),
            purpose: find(&|n| {
                n.starts_with("verwendungszweck") || n == "purpose" || n == "description"
            }),
        })
    }
}

/// Parses `dd.mm.yy` and `dd.mm.yyyy` dates.
pub fn parse_date(raw: &str) -> Option<NaiveDate> {
    let mut parts = raw.trim().split('.');
    let day: u32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let year_str = parts.next()?;
    if parts.next().is_some() || !matches!(year_str.len(), 2 | 4) {
        return None;
    }
    let mut year: i32 = year_str.parse().ok()?;
    if year_str.len() == 2 {
        year += 2000;
    }
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Picks `;` or `,` depending on which one dominates the first lines.
fn sniff_delimiter(content: &str) -> u8 {
    let (mut semicolon_lines, mut comma_lines) = (0, 0);
    for line in content.lines().filter(|l| !l.trim().is_empty()).take(20) {
        let semicolons = line.matches(';').count();
        let commas = line.matches(',').count();
        if semicolons > commas {
            semicolon_lines += 1;
        } else if commas > semicolons {
            comma_lines += 1;
        }
    }
    if comma_lines > semicolon_lines {
        b','
    } else {
        b';'
    }
}

/// Decodes UTF-8 (with or without BOM), falling back to ISO-8859-1 which
/// older DKB exports use.
fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT_EXPORT: &str = "\
\"Girokonto\";\"DE00 1203 0000 0000 0000 00\"
\"Zeitraum:\";\"01.10.2024 - 31.10.2024\"
\"Kontostand vom 31.10.2024:\";\"1.234,56 €\"
\"\"
\"Buchungsdatum\";\"Wertstellung\";\"Status\";\"Zahlungspflichtige*r\";\"Zahlungsempfänger*in\";\"Verwendungszweck\";\"Umsatztyp\";\"IBAN\";\"Betrag (€)\";\"Gläubiger-ID\";\"Mandatsreferenz\";\"Kundenreferenz\"
\"15.10.24\";\"15.10.24\";\"Gebucht\";\"Max Mustermann\";\"REWE Markt GmbH\";\"Einkauf, Filiale 123\";\"Ausgang\";\"DE00\";\"-1.023,45\";\"\";\"\";\"\"
\"01.10.24\";\"01.10.24\";\"Gebucht\";\"ACME GmbH\";\"Max Mustermann\";\"Gehalt Oktober\";\"Eingang\";\"DE00\";\"2.500,00\";\"\";\"\";\"\"
\"02.10.24\";\"02.10.24\";\"Gebucht\";\"Max\";\"Shop\";\"x\";\"Ausgang\";\"DE00\";\"kaputt\";\"\";\"\";\"\"
";

    #[test]
    fn parses_current_dkb_export() {
        let statement = parse_str(CURRENT_EXPORT).unwrap();
        assert_eq!(statement.transactions.len(), 2);

        let rewe = &statement.transactions[0];
        assert_eq!(rewe.date, NaiveDate::from_ymd_opt(2024, 10, 15).unwrap());
        assert_eq!(rewe.recipient, "REWE Markt GmbH");
        assert_eq!(rewe.purpose, "Einkauf, Filiale 123");
        assert_eq!(rewe.amount, Amount::from_cents(-102_345));
        assert_eq!(rewe.counterparty(), "REWE Markt GmbH");

        let salary = &statement.transactions[1];
        assert_eq!(salary.counterparty(), "ACME GmbH");

        assert_eq!(statement.skipped.len(), 1);
        assert_eq!(statement.skipped[0].line, 8);
    }

    #[test]
    fn parses_headerless_comma_file() {
        let content = "15.10.24,15.10.24,Booked,Me,\"PayPal Europe, Luxembourg\",\"Order 1\",Outgoing,LU89,\"-8,99\",X,Y,Z\n";
        let statement = parse_str(content).unwrap();
        assert_eq!(statement.transactions.len(), 1);
        assert_eq!(
            statement.transactions[0].recipient,
            "PayPal Europe, Luxembourg"
        );
        assert_eq!(statement.transactions[0].amount, Amount::from_cents(-899));
    }

    #[test]
    fn parses_legacy_export() {
        let content = "\
\"Kontonummer:\";\"DE00 / Girokonto\";
\"Buchungstag\";\"Wertstellung\";\"Buchungstext\";\"Auftraggeber / Begünstigter\";\"Verwendungszweck\";\"Kontonummer\";\"BLZ\";\"Betrag (EUR)\";
\"03.01.2022\";\"03.01.2022\";\"Lastschrift\";\"Stadtwerke\";\"Strom Januar\";\"DE00\";\"BIC\";\"-45,00\";
";
        let statement = parse_str(content).unwrap();
        let tx = &statement.transactions[0];
        assert_eq!(tx.date, NaiveDate::from_ymd_opt(2022, 1, 3).unwrap());
        assert_eq!(tx.recipient, "Stadtwerke");
        assert_eq!(tx.purpose, "Strom Januar");
        assert_eq!(tx.amount, Amount::from_cents(-4500));
    }

    #[test]
    fn decodes_latin1() {
        assert_eq!(decode(b"Begr\xfc\xdfung"), "Begrüßung");
        assert_eq!(decode("\u{feff}ä".as_bytes()), "ä");
    }

    #[test]
    fn dates() {
        assert_eq!(
            parse_date("15.10.24"),
            NaiveDate::from_ymd_opt(2024, 10, 15)
        );
        assert_eq!(parse_date("1.2.2023"), NaiveDate::from_ymd_opt(2023, 2, 1));
        assert_eq!(parse_date("31.02.24"), None);
        assert_eq!(parse_date("Zeitraum:"), None);
        assert_eq!(parse_date("01.10.2024 - 31.10.2024"), None);
    }
}
