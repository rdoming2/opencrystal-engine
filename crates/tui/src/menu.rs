use std::io;

use crossterm::event::{self, Event};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::dialog::confirm_quit;
use crate::input::{Action, InputBindings};
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

#[derive(Clone, Debug)]
pub struct ContentMenuEntry {
    pub label: String,
    pub title: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub path: String,
    pub enabled: bool,
    pub error: Option<String>,
}

pub fn run_content_menu(
    session: &mut TuiSession,
    bindings: &InputBindings,
    entries: &[ContentMenuEntry],
) -> io::Result<Option<usize>> {
    if entries.is_empty() {
        return Ok(None);
    }
    let mut selected = first_enabled_content(entries).unwrap_or(0);
    loop {
        session.terminal_mut().draw(|frame| {
            draw_content_frame(frame, entries, selected);
        })?;

        if let Event::Key(key) = event::read()? {
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        selected = move_content_selection(selected, entries, -1);
                    }
                    Action::MoveDown => {
                        selected = move_content_selection(selected, entries, 1);
                    }
                    Action::Confirm => {
                        if entries
                            .get(selected)
                            .map(|entry| entry.enabled)
                            .unwrap_or(false)
                        {
                            return Ok(Some(selected));
                        }
                    }
                    Action::Cancel | Action::Menu => return Ok(None),
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_content_frame(frame, entries, selected)
                        })? {
                            return Ok(None);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
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

fn draw_content_frame(frame: &mut Frame, entries: &[ContentMenuEntry], selected: usize) {
    let size = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(size);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(layout[1]);

    let header = Paragraph::new("Select Content")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(header, layout[0]);

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
                style = if entry.enabled {
                    style.fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    style.fg(Color::DarkGray).add_modifier(Modifier::BOLD)
                };
            }
            let prefix = if is_selected { "> " } else { "  " };
            Line::from(Span::styled(format!("{}{}", prefix, entry.label), style))
        })
        .collect::<Vec<_>>();

    let menu_panel = Paragraph::new(menu_lines)
        .block(Block::default().borders(Borders::ALL).title("Games"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(menu_panel, columns[0]);

    let details = entries.get(selected);
    let detail_lines = build_content_details(details);
    let detail_panel = Paragraph::new(detail_lines)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(detail_panel, columns[1]);

    let footer = Paragraph::new("Confirm: select  Cancel: back")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(footer, layout[2]);
}

fn build_content_details(entry: Option<&ContentMenuEntry>) -> Vec<Line<'_>> {
    let Some(entry) = entry else {
        return vec![Line::from(Span::styled(
            "No content available",
            Style::default().fg(Color::DarkGray),
        ))];
    };

    let mut lines = Vec::new();
    let title_style = if entry.enabled {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    };
    lines.push(Line::from(vec![
        Span::styled("Title: ", Style::default().fg(Color::White)),
        Span::styled(entry.title.as_str(), title_style),
    ]));

    if let Some(author) = entry
        .author
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(Line::from(vec![
            Span::styled("Author: ", Style::default().fg(Color::White)),
            Span::styled(author.as_str(), Style::default().fg(Color::Cyan)),
        ]));
    }

    if let Some(description) = entry
        .description
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(Span::styled(
            description.as_str(),
            Style::default().fg(Color::White),
        )));
    }

    lines.push(Line::from(vec![
        Span::styled("Folder: ", Style::default().fg(Color::White)),
        Span::styled(entry.path.as_str(), Style::default().fg(Color::DarkGray)),
    ]));

    if let Some(error) = entry
        .error
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(Line::from(Span::styled(
            "Error:",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            error.as_str(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
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

pub fn render_panel_line(line: &MenuPanelLine) -> Line<'_> {
    let spans = line
        .spans
        .iter()
        .map(|span| Span::styled(span.text.as_str(), panel_span_style(span.style)))
        .collect::<Vec<_>>();
    Line::from(spans)
}

pub fn panel_span_style(style: PanelSpanStyle) -> Style {
    match style {
        PanelSpanStyle::Normal => Style::default().fg(Color::White),
        PanelSpanStyle::Highlight => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        PanelSpanStyle::Muted => Style::default().fg(Color::DarkGray),
        PanelSpanStyle::Accent => Style::default().fg(Color::Cyan),
    }
}

pub fn right_panel_inner_size(
    menu_ui: &MenuUiFile,
    terminal_width: u16,
    terminal_height: u16,
    stats_lines: Option<usize>,
) -> (u16, u16) {
    let (_left_percent, right_percent) = menu_layout_percentages(&menu_ui.layout);
    let right_width = terminal_width
        .saturating_mul(right_percent)
        .saturating_div(100)
        .max(1);
    let mut right_height = terminal_height.saturating_sub(1).max(1);
    if let Some(lines) = stats_lines {
        right_height = right_height.saturating_sub(lines as u16 + 2).max(1);
    }
    let inner_width = right_width.saturating_sub(2).max(1);
    let inner_height = right_height.saturating_sub(2).max(1);
    (inner_width, inner_height)
}

fn first_enabled_content(entries: &[ContentMenuEntry]) -> Option<usize> {
    entries.iter().position(|entry| entry.enabled)
}

fn move_content_selection(current: usize, entries: &[ContentMenuEntry], direction: i32) -> usize {
    if entries.is_empty() {
        return 0;
    }
    let mut index = current.min(entries.len().saturating_sub(1));
    let mut remaining = entries.len();
    while remaining > 0 {
        if direction < 0 {
            index = index.saturating_sub(1);
        } else {
            index = (index + 1).min(entries.len().saturating_sub(1));
        }
        if entries[index].enabled {
            return index;
        }
        remaining -= 1;
    }
    current
}
