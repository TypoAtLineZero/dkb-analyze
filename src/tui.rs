//! Interactive terminal dashboard built with ratatui.
//!
//! ```text
//! ┌ dkb-analyze · Monthly ───────────────────────────────────────────┐
//! │ Oct 24   Income …   Expenses …   Balance …   Transactions …      │
//! ├ Income vs. expenses ─────────────────────────────────────────────┤
//! │   ▆▆ ▂▂   ██ ▅▅   ▇▇ ▆▆                                          │
//! ├ Categories ──────────────┬ Transactions ─────────────────────────┤
//! │ Groceries ███████        │ 15.10.2024  REWE   Groceries  -23,45 │
//! └──────────────────────────┴───────────────────────────────────────┘
//! ```

use std::io;

use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Cell, Paragraph, Row, Table, TableState};

use crate::analysis::{Analysis, Breakdown, Granularity, Period, analyze};
use crate::categories::{Categorizer, Label};
use crate::money::Amount;
use crate::report::truncate;
use crate::transaction::Transaction;

const INCOME_COLOR: Color = Color::Green;
const EXPENSE_COLOR: Color = Color::Red;
const ACCENT: Color = Color::Cyan;
const PALETTE: [Color; 6] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Blue,
    Color::LightRed,
    Color::LightGreen,
];

/// Bar geometry of the income/expense chart.
const BAR_WIDTH: u16 = 6;
const BAR_GAP: u16 = 1;
const GROUP_GAP: u16 = 3;

/// Opens the dashboard and blocks until the user quits.
pub fn run(
    transactions: &[Transaction],
    categorizer: &Categorizer,
    granularity: Granularity,
) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new(transactions, categorizer, granularity).run(&mut terminal);
    ratatui::restore();
    result
}

/// Dashboard state.
pub struct App<'a> {
    transactions: &'a [Transaction],
    categorizer: &'a Categorizer,
    analysis: Analysis,
    /// Index into `analysis.periods`; `None` shows all periods combined.
    selected: Option<usize>,
    /// Indices into `transactions` shown in the table, newest first.
    visible: Vec<usize>,
    table: TableState,
    quit: bool,
}

impl<'a> App<'a> {
    pub fn new(
        transactions: &'a [Transaction],
        categorizer: &'a Categorizer,
        granularity: Granularity,
    ) -> Self {
        let mut app = App {
            transactions,
            categorizer,
            analysis: analyze(transactions, categorizer, granularity),
            selected: None,
            visible: Vec::new(),
            table: TableState::default(),
            quit: false,
        };
        app.select(app.analysis.periods.len().checked_sub(1));
        app
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            }
        }
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        let last = self.analysis.periods.len().checked_sub(1);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Left | KeyCode::Char('h') => {
                self.select(self.selected.map(|i| i.saturating_sub(1)).or(last));
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.select(
                    self.selected
                        .map(|i| (i + 1).min(last.unwrap_or(0)))
                        .or(last),
                );
            }
            KeyCode::Char('a') => self.select(if self.selected.is_some() { None } else { last }),
            KeyCode::Tab | KeyCode::Char('g') => {
                self.set_granularity(self.analysis.granularity.next())
            }
            KeyCode::Char('w') => self.set_granularity(Granularity::Weekly),
            KeyCode::Char('m') => self.set_granularity(Granularity::Monthly),
            KeyCode::Char('y') => self.set_granularity(Granularity::Yearly),
            KeyCode::Down | KeyCode::Char('j') => self.table.scroll_down_by(1),
            KeyCode::Up | KeyCode::Char('k') => self.table.scroll_up_by(1),
            KeyCode::PageDown => self.table.scroll_down_by(10),
            KeyCode::PageUp => self.table.scroll_up_by(10),
            KeyCode::Home => self.table.select_first(),
            KeyCode::End => self.table.select_last(),
            _ => {}
        }
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    fn set_granularity(&mut self, granularity: Granularity) {
        // Keep looking at the same point in time after regrouping: the newest
        // transaction of the selected period decides the new period.
        let anchor = self
            .selected
            .and_then(|_| self.visible.first())
            .map(|&i| self.transactions[i].date);
        self.analysis = analyze(self.transactions, self.categorizer, granularity);
        let index = anchor.and_then(|date| {
            let period = granularity.period_of(date);
            self.analysis.periods.iter().position(|(p, _)| *p == period)
        });
        self.select(index);
    }

    fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        let period = self.selected_period();
        let granularity = self.analysis.granularity;
        self.visible = (0..self.transactions.len())
            .filter(|&i| {
                period.is_none_or(|p| granularity.period_of(self.transactions[i].date) == p)
            })
            .collect();
        self.visible
            .sort_by_key(|&i| std::cmp::Reverse(self.transactions[i].date));
        self.table = TableState::default().with_selected((!self.visible.is_empty()).then_some(0));
    }

    fn selected_period(&self) -> Option<Period> {
        self.selected.map(|i| self.analysis.periods[i].0)
    }

    fn selected_breakdown(&self) -> &Breakdown {
        self.selected
            .map_or(&self.analysis.overall, |i| &self.analysis.periods[i].1)
    }

    fn selection_label(&self) -> String {
        self.selected_period()
            .map_or_else(|| "All periods".to_string(), Period::short_label)
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let [header, chart, bottom, help] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Percentage(45),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .areas(frame.area());
        let [categories, table] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(bottom);

        self.draw_header(frame, header);
        self.draw_chart(frame, chart);
        self.draw_categories(frame, categories);
        self.draw_table(frame, table);
        draw_help(frame, help);
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let range = self.analysis.range.map_or_else(String::new, |(from, to)| {
            format!(" {} – {} ", from.format("%d.%m.%Y"), to.format("%d.%m.%Y"))
        });
        let block = Block::bordered()
            .title(Line::from(vec![
                " dkb-analyze ".bold().fg(ACCENT),
                format!("· {} ", self.analysis.granularity).into(),
            ]))
            .title_top(Line::from(range).right_aligned());

        let b = self.selected_breakdown();
        let line = Line::from(vec![
            format!(" {:<12}", self.selection_label()).bold(),
            "Income ".dim(),
            b.income.to_string().fg(INCOME_COLOR),
            "   Expenses ".dim(),
            b.expenses.to_string().fg(EXPENSE_COLOR),
            "   Balance ".dim(),
            amount_span(b.balance()).bold(),
            "   Transactions ".dim(),
            b.transactions.to_string().into(),
            "   Uncategorized ".dim(),
            format!("{} ({})", b.uncategorized, b.uncategorized_amount).into(),
        ]);
        frame.render_widget(Paragraph::new(line).block(block), area);
    }

    fn draw_chart(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(Line::from(vec![
            " ".into(),
            "■ Income".fg(INCOME_COLOR),
            "  ".into(),
            "■ Expenses".fg(EXPENSE_COLOR),
            format!("  per {} (€) ", period_noun(self.analysis.granularity)).into(),
        ]));
        let periods = &self.analysis.periods;
        if periods.is_empty() {
            frame.render_widget(Paragraph::new(" No transactions.").block(block), area);
            return;
        }

        // Show the window of periods that fits, keeping the selection visible.
        let inner_width = area.width.saturating_sub(2);
        let group_width = 2 * BAR_WIDTH + BAR_GAP + GROUP_GAP;
        let capacity = usize::from((inner_width + GROUP_GAP) / group_width).max(1);
        let focus = self.selected.unwrap_or(periods.len() - 1);
        let start = (focus + 1).saturating_sub(capacity);
        let end = (start + capacity).min(periods.len());

        let mut chart = BarChart::default()
            .block(block)
            .bar_width(BAR_WIDTH)
            .bar_gap(BAR_GAP)
            .group_gap(GROUP_GAP);
        for (index, (period, breakdown)) in periods.iter().enumerate().take(end).skip(start) {
            let is_selected = self.selected == Some(index);
            let dim = self.selected.is_some() && !is_selected;
            let style = |color: Color| {
                if dim {
                    Style::new().fg(color).add_modifier(Modifier::DIM)
                } else {
                    Style::new().fg(color)
                }
            };
            let bar = |amount: Amount, color: Color| {
                Bar::default()
                    .value(amount.abs_euros())
                    .text_value(amount.abs().compact())
                    .style(style(color))
                    .value_style(style(color).add_modifier(Modifier::REVERSED))
            };
            let label = if is_selected {
                Line::from(period.short_label().bold().fg(ACCENT).underlined())
            } else {
                Line::from(period.short_label())
            };
            chart = chart.data(BarGroup::default().label(label.centered()).bars(&[
                bar(breakdown.income, INCOME_COLOR),
                bar(breakdown.expenses, EXPENSE_COLOR),
            ]));
        }
        frame.render_widget(chart, area);
    }

    fn draw_categories(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(format!(
            " Expenses by category · {} ",
            self.selection_label()
        ));
        let categories = &self.selected_breakdown().categories;
        if categories.is_empty() {
            frame.render_widget(Paragraph::new(" No expenses.").block(block), area);
            return;
        }
        // The label carries name, amount and share; the bar itself stays free
        // of text so that small bars remain visible.
        let total = self
            .selected_breakdown()
            .expenses
            .cents()
            .unsigned_abs()
            .max(1) as f64;
        let name_width = categories
            .iter()
            .map(|c| c.name.chars().count())
            .max()
            .unwrap_or(0);
        let name_width = name_width.min(usize::from(area.width / 3).max(6));
        let bars: Vec<Bar> = categories
            .iter()
            .enumerate()
            .map(|(i, category)| {
                let share = category.amount.cents().unsigned_abs() as f64 * 100.0 / total;
                let label = format!(
                    "{:<name_width$} {:>12} {:>3.0}%",
                    truncate(&category.name, name_width),
                    category.amount.to_string(),
                    share
                );
                Bar::default()
                    .label(Line::from(label))
                    .value(category.amount.abs_euros())
                    .text_value(String::new())
                    .style(Style::new().fg(PALETTE[i % PALETTE.len()]))
            })
            .collect();
        let chart = BarChart::default()
            .block(block)
            .direction(Direction::Horizontal)
            .bar_width(1)
            .bar_gap(0)
            .data(BarGroup::default().bars(&bars));
        frame.render_widget(chart, area);
    }

    fn draw_table(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title(format!(
            " Transactions · {} ({}) ",
            self.selection_label(),
            self.visible.len()
        ));
        let rows = self.visible.iter().map(|&i| {
            let tx = &self.transactions[i];
            let label = self.categorizer.classify(tx);
            let category = match label {
                Label::Uncategorized => Span::from(label.name()).dim().italic(),
                Label::Income => Span::from(label.name()).fg(INCOME_COLOR),
                Label::Category(name) => Span::from(name),
            };
            Row::new([
                Cell::from(tx.date.format("%d.%m.%Y").to_string()),
                Cell::from(tx.counterparty().to_string()),
                Cell::from(category),
                Cell::from(Line::from(amount_span(tx.amount)).right_aligned()),
            ])
        });
        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Fill(1),
                Constraint::Length(14),
                Constraint::Length(14),
            ],
        )
        .header(
            Row::new([
                Cell::from("Date"),
                Cell::from("Counterparty"),
                Cell::from("Category"),
                Cell::from(Line::from("Amount").right_aligned()),
            ])
            .style(Style::new().bold().fg(ACCENT)),
        )
        .block(block)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
        frame.render_stateful_widget(table, area, &mut self.table);
    }
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let keys = [
        ("←/→", "period"),
        ("a", "all/one"),
        ("Tab", "week/month/year"),
        ("↑/↓", "scroll"),
        ("q", "quit"),
    ];
    let spans: Vec<Span> = keys
        .iter()
        .flat_map(|(key, action)| {
            [
                format!(" {key} ").bold().fg(ACCENT),
                format!("{action}  ").dim(),
            ]
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn amount_span(amount: Amount) -> Span<'static> {
    let color = if amount.is_negative() {
        EXPENSE_COLOR
    } else {
        INCOME_COLOR
    };
    amount.to_string().fg(color)
}

fn period_noun(granularity: Granularity) -> &'static str {
    match granularity {
        Granularity::Weekly => "week",
        Granularity::Monthly => "month",
        Granularity::Yearly => "year",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn transactions() -> Vec<Transaction> {
        let tx = |m: u32, d: u32, recipient: &str, cents: i64| Transaction {
            date: NaiveDate::from_ymd_opt(2024, m, d).unwrap(),
            payer: "ACME GmbH".into(),
            recipient: recipient.into(),
            purpose: String::new(),
            amount: Amount::from_cents(cents),
        };
        vec![
            tx(10, 1, "Me", 250_000),
            tx(10, 3, "REWE Markt", -4_250),
            tx(11, 1, "Me", 250_000),
            tx(11, 4, "Mystery Shop", -9_999),
            tx(12, 30, "Deutsche Bahn", -12_900),
        ]
    }

    fn render(app: &mut App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn renders_latest_period_by_default() {
        let txs = transactions();
        let categorizer = Categorizer::default();
        let mut app = App::new(&txs, &categorizer, Granularity::Monthly);
        let screen = render(&mut app);
        assert!(screen.contains("Dec 24"));
        assert!(screen.contains("Deutsche Bahn"));
        assert!(screen.contains("Transportation"));
        assert!(!screen.contains("Mystery Shop"), "only December is listed");
    }

    #[test]
    fn navigation() {
        let txs = transactions();
        let categorizer = Categorizer::default();
        let mut app = App::new(&txs, &categorizer, Granularity::Monthly);

        press(&mut app, KeyCode::Left);
        assert!(render(&mut app).contains("Mystery Shop"));

        press(&mut app, KeyCode::Char('a'));
        assert_eq!(app.visible.len(), txs.len());
        assert!(render(&mut app).contains("All periods"));

        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Char('y'));
        assert_eq!(app.analysis.periods.len(), 1);
        assert_eq!(app.selected, Some(0));

        press(&mut app, KeyCode::Char('w'));
        assert!(render(&mut app).contains("Weekly"));

        press(&mut app, KeyCode::Char('q'));
        assert!(app.should_quit());
    }

    #[test]
    fn renders_without_transactions() {
        let categorizer = Categorizer::default();
        let mut app = App::new(&[], &categorizer, Granularity::Monthly);
        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Down);
        assert!(render(&mut app).contains("No transactions."));
    }
}
