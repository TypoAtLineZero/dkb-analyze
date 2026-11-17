//! Exact money arithmetic.
//!
//! Amounts are stored as integer cents, so summing hundreds of bookings never
//! accumulates floating point rounding errors.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg};
use std::str::FromStr;

/// A signed amount of money in euro cents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(i64);

impl Amount {
    pub const ZERO: Amount = Amount(0);

    pub const fn from_cents(cents: i64) -> Self {
        Amount(cents)
    }

    pub const fn cents(self) -> i64 {
        self.0
    }

    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    pub const fn abs(self) -> Self {
        Amount(self.0.abs())
    }

    /// Whole euros of the absolute value, used for chart bar heights.
    pub const fn abs_euros(self) -> u64 {
        self.0.unsigned_abs() / 100
    }

    /// Compact representation for narrow spaces such as chart bars:
    /// `850`, `1.2k`, `12k`, `1.5M`.
    pub fn compact(self) -> String {
        let euros = self.0.unsigned_abs() as f64 / 100.0;
        let sign = if self.is_negative() { "-" } else { "" };
        let body = if euros >= 1_000_000.0 {
            format!("{:.1}M", euros / 1_000_000.0)
        } else if euros >= 10_000.0 {
            format!("{:.0}k", euros / 1_000.0)
        } else if euros >= 1_000.0 {
            format!("{:.1}k", euros / 1_000.0)
        } else {
            format!("{euros:.0}")
        };
        format!("{sign}{body}")
    }
}

/// Error returned when a string is not a valid German-formatted amount.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid amount {0:?}")]
pub struct ParseAmountError(String);

impl FromStr for Amount {
    type Err = ParseAmountError;

    /// Parses amounts as they appear in DKB exports, e.g. `-8,99`, `"-75"`,
    /// `1.234,56`, `+12,5` or `-1.234,56 €`.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let err = || ParseAmountError(input.to_string());

        let cleaned: String = input
            .trim()
            .trim_matches('"')
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '€')
            .collect();

        let (negative, unsigned) = if let Some(rest) = cleaned.strip_prefix(['-', '−']) {
            (true, rest)
        } else {
            (false, cleaned.strip_prefix('+').unwrap_or(&cleaned))
        };

        // German format: '.' groups thousands, ',' separates the decimals.
        let digits = unsigned.replace('.', "");
        let (int_part, frac_part) = digits.split_once(',').unwrap_or((&digits, ""));

        let is_digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
        if (int_part.is_empty() && frac_part.is_empty())
            || !is_digits(int_part)
            || !is_digits(frac_part)
            || frac_part.len() > 2
        {
            return Err(err());
        }

        let euros: i64 = if int_part.is_empty() {
            0
        } else {
            int_part.parse().map_err(|_| err())?
        };
        let cents: i64 = match frac_part.len() {
            0 => 0,
            1 => frac_part.parse::<i64>().map_err(|_| err())? * 10,
            _ => frac_part.parse().map_err(|_| err())?,
        };

        let total = euros
            .checked_mul(100)
            .and_then(|c| c.checked_add(cents))
            .ok_or_else(err)?;
        Ok(Amount(if negative { -total } else { total }))
    }
}

impl fmt::Display for Amount {
    /// German formatting: `-1.234,56 €`. Honors width/alignment flags.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.0.unsigned_abs();
        let euros = (abs / 100).to_string();
        let mut grouped = String::with_capacity(euros.len() + euros.len() / 3);
        for (i, digit) in euros.chars().enumerate() {
            if i > 0 && (euros.len() - i).is_multiple_of(3) {
                grouped.push('.');
            }
            grouped.push(digit);
        }
        let sign = if self.is_negative() { "-" } else { "" };
        f.pad(&format!("{sign}{grouped},{:02} €", abs % 100))
    }
}

impl Add for Amount {
    type Output = Amount;
    fn add(self, rhs: Amount) -> Amount {
        Amount(self.0 + rhs.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Amount) {
        self.0 += rhs.0;
    }
}

impl Neg for Amount {
    type Output = Amount;
    fn neg(self) -> Amount {
        Amount(-self.0)
    }
}

impl Sum for Amount {
    fn sum<I: Iterator<Item = Amount>>(iter: I) -> Amount {
        iter.fold(Amount::ZERO, Add::add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Amount {
        s.parse().unwrap()
    }

    #[test]
    fn parses_dkb_formats() {
        assert_eq!(parse("-8,99"), Amount::from_cents(-899));
        assert_eq!(parse("\"-11,9\""), Amount::from_cents(-1190));
        assert_eq!(parse("-75"), Amount::from_cents(-7500));
        assert_eq!(parse("1.234,56"), Amount::from_cents(123_456));
        assert_eq!(parse("+12,05"), Amount::from_cents(1205));
        assert_eq!(parse("-1.234,56 €"), Amount::from_cents(-123_456));
        assert_eq!(parse(",5"), Amount::from_cents(50));
        assert_eq!(parse("0"), Amount::ZERO);
    }

    #[test]
    fn rejects_garbage() {
        for bad in ["", "-", "abc", "1,234", "12a", "1,2,3", "€"] {
            assert!(bad.parse::<Amount>().is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn displays_german_format() {
        assert_eq!(
            Amount::from_cents(-123_456_789).to_string(),
            "-1.234.567,89 €"
        );
        assert_eq!(Amount::from_cents(5).to_string(), "0,05 €");
        assert_eq!(Amount::from_cents(100_000).to_string(), "1.000,00 €");
        assert_eq!(format!("{:>12}", Amount::from_cents(-899)), "     -8,99 €");
    }

    #[test]
    fn compact_format() {
        assert_eq!(Amount::from_cents(85_049).compact(), "850");
        assert_eq!(Amount::from_cents(-123_400).compact(), "-1.2k");
        assert_eq!(Amount::from_cents(1_250_000).compact(), "12k");
    }

    #[test]
    fn arithmetic() {
        let total: Amount = [-899, -1190, 2500]
            .map(Amount::from_cents)
            .into_iter()
            .sum();
        assert_eq!(total, Amount::from_cents(411));
        assert_eq!(-total, Amount::from_cents(-411));
    }
}
