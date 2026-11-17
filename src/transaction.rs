//! The booking model shared by parser, analysis and UI.

use chrono::NaiveDate;

use crate::money::Amount;

/// A single booking from an account statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    /// Posting date ("Buchungsdatum").
    pub date: NaiveDate,
    /// Who paid ("Zahlungspflichtige*r").
    pub payer: String,
    /// Who received the money ("Zahlungsempfänger*in").
    pub recipient: String,
    /// Purpose / reference text ("Verwendungszweck").
    pub purpose: String,
    /// Signed amount: negative for spending, positive for income.
    pub amount: Amount,
}

impl Transaction {
    /// The other party of the booking: the payer for income, the recipient
    /// for spending. Falls back to the other field when one is empty.
    pub fn counterparty(&self) -> &str {
        let (primary, fallback) = if self.amount.is_positive() {
            (&self.payer, &self.recipient)
        } else {
            (&self.recipient, &self.payer)
        };
        if primary.trim().is_empty() {
            fallback.trim()
        } else {
            primary.trim()
        }
    }
}
