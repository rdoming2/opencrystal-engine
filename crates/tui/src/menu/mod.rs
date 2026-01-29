use std::io;

use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::session::TuiSession;
use crate::ui::{MenuLayout, MenuUiFile};

#[derive(Clone, Copy, Debug)]
pub enum MenuPane {
    List,
    Detail,
}

#[derive(Clone, Debug)]
pub struct MenuEntryView {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct MenuPanelView {
    pub title: String,
    pub lines: Vec<MenuPanelLine>,
}

#[derive(Clone, Debug)]
pub struct MenuPanelLine {
    pub spans: Vec<MenuPanelSpan>,
}

#[derive(Clone, Debug)]
pub struct MenuPanelSpan {
    pub text: String,
    pub style: PanelSpanStyle,
}

#[derive(Clone, Copy, Debug)]
pub enum PanelSpanStyle {
    Normal,
    Highlight,
    Muted,
    Accent,
}

#[derive(Clone, Debug)]
pub struct InventoryLine {
    pub label: String,
    pub count: i32,
    pub enabled: bool,
    pub equipped_by: Option<String>,
}

#[derive(Clone, Debug)]
pub struct InventoryHeader {
    pub title: String,
    pub filters: Vec<(String, bool)>,
    pub sort_label: String,
}

pub fn draw_menu(
    session: &mut TuiSession,
    menu_ui: &MenuUiFile,
    entries: &[MenuEntryView],
    selected: usize,
    focus: MenuPane,
    right_panel: &MenuPanelView,
    stats_panel: Option<&MenuPanelView>,
    footer_text: &str,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_menu_frame(
                frame,
                menu_ui,
                entries,
                selected,
                focus,
                right_panel,
                stats_panel,
                footer_text,
            );
        })
        .map(|_| ())
}

pub fn draw_inventory(
    session: &mut TuiSession,
    header: &InventoryHeader,
    entries: &[InventoryLine],
    selected: usize,
    right_panel: &MenuPanelView,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_inventory_frame(frame, header, entries, selected, right_panel);
        })
        .map(|_| ())
}

pub fn draw_menu_frame(
    frame: &mut Frame,
    menu_ui: &MenuUiFile,
    entries: &[MenuEntryView],
    selected: usize,
    focus: MenuPane,
    right_panel: &MenuPanelView,
    stats_panel: Option<&MenuPanelView>,
    footer_text: &str,
) {
    let size = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);
    let (left_percent, right_percent) = menu_layout_percentages(&menu_ui.layout);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_percent),
            Constraint::Percentage(right_percent),
        ])
        .split(layout[0]);

    let menu_lines = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let is_selected = index == selected;
            let mut style = if entry.enabled {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            if is_selected {
                style = match focus {
                    MenuPane::List => style.fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    MenuPane::Detail => style.fg(Color::Cyan),
                };
            }
            let prefix = if is_selected && matches!(focus, MenuPane::List) {
                "> "
            } else {
                "  "
            };
            Line::from(Span::styled(format!("{}{}", prefix, entry.label), style))
        })
        .collect::<Vec<_>>();

    let menu_panel = Paragraph::new(menu_lines)
        .block(Block::default().borders(Borders::ALL).title("Menu"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(menu_panel, columns[0]);

    match stats_panel {
        Some(stats) => {
            let right_column = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(stats.lines.len() as u16 + 2),
                ])
                .split(columns[1]);

            let detail_lines = right_panel
                .lines
                .iter()
                .map(render_panel_line)
                .collect::<Vec<_>>();
            let detail_panel = Paragraph::new(detail_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(right_panel.title.as_str()),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(detail_panel, right_column[0]);

            let stats_lines = stats
                .lines
                .iter()
                .map(render_panel_line)
                .collect::<Vec<_>>();
            let stats_panel_widget = Paragraph::new(stats_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(stats.title.as_str()),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(stats_panel_widget, right_column[1]);
        }
        None => {
            let detail_lines = right_panel
                .lines
                .iter()
                .map(render_panel_line)
                .collect::<Vec<_>>();
            let detail_panel = Paragraph::new(detail_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(right_panel.title.as_str()),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(detail_panel, columns[1]);
        }
    }

    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(footer, layout[1]);
}

pub fn draw_inventory_frame(
    frame: &mut Frame,
    header: &InventoryHeader,
    entries: &[InventoryLine],
    selected: usize,
    right_panel: &MenuPanelView,
) {
    let size = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(layout[0]);

    let mut filter_spans = Vec::new();
    filter_spans.push(Span::raw("Filter: "));
    for (index, (label, active)) in header.filters.iter().enumerate() {
        if index > 0 {
            filter_spans.push(Span::raw(" | "));
        }
        let style = if *active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        filter_spans.push(Span::styled(label.as_str(), style));
    }
    filter_spans.push(Span::raw("   Sort: "));
    filter_spans.push(Span::styled(
        header.sort_label.as_str(),
        Style::default().fg(Color::Cyan),
    ));

    let mut left_lines = Vec::new();
    left_lines.push(Line::from(filter_spans));
    left_lines.extend(entries.iter().enumerate().map(|(index, entry)| {
        let is_selected = index == selected;
        let mut style = if entry.enabled {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if entry.equipped_by.is_some() {
            style = style.fg(Color::Cyan);
        }
        if is_selected {
            style = style.add_modifier(Modifier::BOLD);
        }
        let prefix = if is_selected { "> " } else { "  " };
        let mut label = format!("{}{} x{}", prefix, entry.label, entry.count);
        if let Some(owner) = &entry.equipped_by {
            label.push_str(" ");
            label.push_str("(");
            label.push_str(owner);
            label.push_str(")");
        }
        Line::from(Span::styled(label, style))
    }));

    let left_panel = Paragraph::new(left_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(header.title.as_str()),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(left_panel, columns[0]);

    let detail_lines = right_panel
        .lines
        .iter()
        .map(render_panel_line)
        .collect::<Vec<_>>();
    let detail_panel = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(right_panel.title.as_str()),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(detail_panel, columns[1]);

    let footer = Paragraph::new("Confirm: use/equip  Cancel: back  Pause: sort")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(footer, layout[1]);
}

fn menu_layout_percentages(layout: &MenuLayout) -> (u16, u16) {
    let total = layout.left_width_ratio + layout.right_width_ratio;
    let left_ratio = if total > 0.0 {
        layout.left_width_ratio / total
    } else {
        0.4
    };
    let left_percent = (left_ratio * 100.0).round().clamp(10.0, 90.0) as u16;
    let right_percent = 100u16.saturating_sub(left_percent).max(1);
    (left_percent, right_percent)
}

fn render_panel_line(line: &MenuPanelLine) -> Line<'_> {
    let spans = line
        .spans
        .iter()
        .map(|span| Span::styled(span.text.as_str(), panel_span_style(span.style)))
        .collect::<Vec<_>>();
    Line::from(spans)
}

fn panel_span_style(style: PanelSpanStyle) -> Style {
    match style {
        PanelSpanStyle::Normal => Style::default().fg(Color::White),
        PanelSpanStyle::Highlight => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        PanelSpanStyle::Muted => Style::default().fg(Color::DarkGray),
        PanelSpanStyle::Accent => Style::default().fg(Color::Cyan),
    }
}
