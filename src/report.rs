//! Plain-text report for the terminal (or a pipe / file).

use std::fmt::Write;

use crate::analysis::Analysis;
use crate::money::Amount;
use crate::recurring::MerchantGroup;

const WIDTH: usize = 72;
const BAR_WIDTH: usize = 20;

/// Renders the analysis as a text report. `uncategorized` are the merchant
/// groups of uncategorized spending, of which the first `limit` are listed.
pub fn render(analysis: &Analysis, uncategorized: &[MerchantGroup], limit: usize) -> String {
    let mut out = String::new();
    // Writing to a String cannot fail.
    let _ = write_report(&mut out, analysis, uncategorized, limit);
    out
}

fn write_report(
    out: &mut String,
    analysis: &Analysis,
    uncategorized: &[MerchantGroup],
    limit: usize,
) -> std::fmt::Result {
    let overall = &analysis.overall;

    let range = analysis.range.map_or_else(String::new, |(from, to)| {
        format!("{} – {}", from.format("%d.%m.%Y"), to.format("%d.%m.%Y"))
    });
    heading(out, "OVERVIEW", &range)?;
    writeln!(out, "  {:<24}{:>16}", "Income", overall.income)?;
    writeln!(out, "  {:<24}{:>16}", "Expenses", overall.expenses)?;
    writeln!(out, "  {:<24}{:>16}", "Balance", overall.balance())?;
    writeln!(out, "  {:<24}{:>16}", "Transactions", overall.transactions)?;
    writeln!(
        out,
        "  {:<24}{:>16}   ({})",
        "Uncategorized", overall.uncategorized, overall.uncategorized_amount
    )?;

    heading(out, "EXPENSES BY CATEGORY", "")?;
    if overall.categories.is_empty() {
        writeln!(out, "  No expenses.")?;
    }
    let total_spent = overall.expenses.cents().unsigned_abs().max(1) as f64;
    let largest = overall
        .categories
        .first()
        .map_or(0, |c| c.amount.cents().unsigned_abs());
    for category in &overall.categories {
        let spent = category.amount.cents().unsigned_abs();
        writeln!(
            out,
            "  {:<22}{:>16} {:>6.1} %  {}",
            truncate(&category.name, 22),
            category.amount,
            spent as f64 * 100.0 / total_spent,
            bar(spent, largest, '█'),
        )?;
    }

    heading(
        out,
        &format!(
            "{} BALANCE",
            analysis.granularity.to_string().to_uppercase()
        ),
        "",
    )?;
    writeln!(
        out,
        "  {:<9}{:>15}{:>16}{:>16}",
        "Period", "Income", "Expenses", "Balance"
    )?;
    let largest_balance = analysis
        .periods
        .iter()
        .map(|(_, b)| b.balance().cents().unsigned_abs())
        .max()
        .unwrap_or(0);
    for (period, breakdown) in &analysis.periods {
        let balance = breakdown.balance();
        let fill = if balance.is_negative() { '░' } else { '█' };
        writeln!(
            out,
            "  {:<9}{:>15}{:>16}{:>16}  {}",
            period.to_string(),
            breakdown.income,
            breakdown.expenses,
            balance,
            bar(balance.cents().unsigned_abs(), largest_balance, fill),
        )?;
    }
    if analysis.periods.len() > 1 {
        let n = analysis.periods.len() as i64;
        let average = Amount::from_cents(overall.balance().cents() / n);
        writeln!(out, "  {:<40}{:>16}", "Average", average)?;
    }

    if !uncategorized.is_empty() && limit > 0 {
        write_uncategorized(out, uncategorized, limit)?;
    }
    writeln!(out, "{}", "═".repeat(WIDTH))
}

fn write_uncategorized(
    out: &mut String,
    groups: &[MerchantGroup],
    limit: usize,
) -> std::fmt::Result {
    heading(
        out,
        "UNCATEGORIZED BY MERCHANT",
        "keywords for categories.toml",
    )?;
    writeln!(
        out,
        "  {:<25}{:<9}{:>5}{:>7}{:>11}{:>13}",
        "Keyword", "Pattern", "Count", "Months", "Ø amount", "Total"
    )?;
    for group in groups.iter().take(limit) {
        let keyword = match &group.keyword {
            Some(keyword) => keyword.clone(),
            None => format!("({})", group.key),
        };
        writeln!(
            out,
            "  {:<25}{:<9}{:>5}{:>7}{:>11}{:>13}",
            truncate(&keyword, 24),
            group.pattern,
            group.count(),
            group.months,
            group.average(),
            group.total,
        )?;
    }
    if groups.len() > limit {
        let rest = &groups[limit..];
        let total: Amount = rest.iter().map(|g| g.total).sum();
        writeln!(out, "  … {} more merchants ({total})", rest.len())?;
    }
    writeln!(
        out,
        "  Run with --suggest-categories to get these as a categories.toml snippet."
    )
}

fn heading(out: &mut String, title: &str, note: &str) -> std::fmt::Result {
    writeln!(out, "{}", "═".repeat(WIDTH))?;
    if note.is_empty() {
        writeln!(out, " {title}")?;
    } else {
        let gap = WIDTH.saturating_sub(title.chars().count() + note.chars().count() + 2);
        writeln!(out, " {title}{}{note}", " ".repeat(gap))?;
    }
    writeln!(out, "{}", "─".repeat(WIDTH))
}

fn bar(value: u64, max: u64, fill: char) -> String {
    if max == 0 {
        return String::new();
    }
    let len = (value as f64 / max as f64 * BAR_WIDTH as f64).round() as usize;
    // Keep tiny non-zero values visible.
    let len = if value > 0 { len.max(1) } else { 0 };
    fill.to_string().repeat(len)
}

/// Shortens `text` to at most `max` characters, marking cuts with `…`.
pub(crate) fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
        cut.push('…');
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_scale() {
        assert_eq!(bar(10, 10, '█').chars().count(), BAR_WIDTH);
        assert_eq!(bar(1, 1000, '█'), "█");
        assert_eq!(bar(0, 10, '█'), "");
        assert_eq!(bar(5, 0, '█'), "");
    }

    #[test]
    fn truncates_by_chars() {
        assert_eq!(truncate("Bäckerei", 10), "Bäckerei");
        assert_eq!(truncate("Bäckerei Müller", 8), "Bäckere…");
    }
}
