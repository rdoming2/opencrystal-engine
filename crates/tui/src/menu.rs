use std::io;

use crossterm::event::{self, Event};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::dialog::confirm_quit;
use crate::input::{Action, InputBindings, is_actionable_key};
use crate::session::TuiSession;
use crate::ui::{MenuLayout, MenuUiFile};
use crate::utils::{centered_rect, palette_color};

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
pub struct StatusCard {
    pub title: String,
    pub lines: Vec<MenuPanelLine>,
}

#[derive(Clone, Debug)]
pub struct StatusScreenView {
    pub title: String,
    pub cards: Vec<StatusCard>,
}

#[derive(Clone, Debug)]
pub struct MenuPanelLine {
    pub spans: Vec<MenuPanelSpan>,
}

#[derive(Clone, Debug)]
pub struct MenuPanelSpan {
    pub text: String,
    pub style: PanelSpanStyle,
    pub palette: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum PanelSpanStyle {
    Normal,
    Highlight,
    Muted,
    Accent,
    Positive,
    Negative,
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
            if !is_actionable_key(&key) {
                continue;
            }
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

pub fn draw_menu_status(
    session: &mut TuiSession,
    menu_ui: &MenuUiFile,
    entries: &[MenuEntryView],
    selected: usize,
    focus: MenuPane,
    status_view: &StatusScreenView,
    footer_text: &str,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_menu_status_frame(
                frame,
                menu_ui,
                entries,
                selected,
                focus,
                status_view,
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

pub fn draw_status_screen(
    session: &mut TuiSession,
    view: &StatusScreenView,
    footer_text: &str,
) -> io::Result<()> {
    session
        .terminal_mut()
        .draw(|frame| {
            draw_status_frame(frame, view, footer_text);
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
    let size = frame.area();
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

    let available_lines = columns[0].height.saturating_sub(2) as usize;
    let page_size = available_lines.max(1);
    let page = if entries.is_empty() {
        0
    } else {
        selected / page_size
    };
    let start = page.saturating_mul(page_size);
    let end = (start + page_size).min(entries.len());
    let menu_lines = entries
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
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

pub fn draw_menu_status_frame(
    frame: &mut Frame,
    menu_ui: &MenuUiFile,
    entries: &[MenuEntryView],
    selected: usize,
    focus: MenuPane,
    status_view: &StatusScreenView,
    footer_text: &str,
) {
    let size = frame.area();
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

    let available_lines = columns[0].height.saturating_sub(2) as usize;
    let page_size = available_lines.max(1);
    let page = if entries.is_empty() {
        0
    } else {
        selected / page_size
    };
    let start = page.saturating_mul(page_size);
    let end = (start + page_size).min(entries.len());
    let menu_lines = entries
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
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

    render_status_cards_in_area(frame, columns[1], status_view);

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
    let size = frame.area();
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

pub fn draw_status_frame(frame: &mut Frame, view: &StatusScreenView, footer_text: &str) {
    let size = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    render_status_cards_in_area(frame, layout[0], view);

    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(footer, layout[1]);
}

pub fn confirm_menu_exit<F>(session: &mut TuiSession, draw_background: F) -> io::Result<bool>
where
    F: Fn(&mut Frame),
{
    loop {
        session.terminal_mut().draw(|frame| {
            draw_background(frame);
            let area = centered_rect(frame.area(), 42, 3);
            frame.render_widget(Clear, area);
            let content = vec![Line::from(Span::raw("Return to title screen? (Y/N)"))];
            let paragraph = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            match key.code {
                crossterm::event::KeyCode::Char('y') | crossterm::event::KeyCode::Char('Y') => {
                    return Ok(true);
                }
                crossterm::event::KeyCode::Char('n') | crossterm::event::KeyCode::Char('N') => {
                    return Ok(false);
                }
                _ => return Ok(false),
            }
        }
    }
}

pub fn show_menu_notice_modal<F>(
    session: &mut TuiSession,
    bindings: &InputBindings,
    draw_background: F,
    message: &str,
) -> io::Result<()>
where
    F: Fn(&mut Frame),
{
    loop {
        session.terminal_mut().draw(|frame| {
            draw_background(frame);
            let width = (message.chars().count() as u16).saturating_add(4).max(24);
            let area = centered_rect(frame.area(), width, 3);
            frame.render_widget(Clear, area);
            let content = vec![Line::from(Span::raw(message))];
            let paragraph = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::Confirm | Action::Cancel | Action::Menu => return Ok(()),
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_background(frame);
                            let width = (message.chars().count() as u16).saturating_add(4).max(24);
                            let area = centered_rect(frame.area(), width, 3);
                            frame.render_widget(Clear, area);
                            let content = vec![Line::from(Span::raw(message))];
                            let paragraph = Paragraph::new(content)
                                .block(Block::default().borders(Borders::ALL))
                                .alignment(Alignment::Center);
                            frame.render_widget(paragraph, area);
                        })? {
                            return Err(io::Error::new(io::ErrorKind::Interrupted, "quit"));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn draw_content_frame(frame: &mut Frame, entries: &[ContentMenuEntry], selected: usize) {
    let size = frame.area();
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

fn status_screen_columns(width: u16) -> usize {
    let min_card_width = 26u16;
    let mut columns = (width / min_card_width).max(1) as usize;
    columns = columns.min(3);
    columns
}

fn render_status_cards_in_area(frame: &mut Frame, area: Rect, view: &StatusScreenView) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(view.title.as_str());
    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);
    let columns = status_screen_columns(inner_area.width);
    let column_constraints = status_column_constraints(columns);
    let column_areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_constraints)
        .split(inner_area);

    let column_cards = assign_status_cards(&view.cards, columns);
    for (column_index, cards) in column_cards.iter().enumerate() {
        let Some(column_area) = column_areas.get(column_index) else {
            continue;
        };
        if cards.is_empty() {
            continue;
        }
        let mut constraints = Vec::with_capacity(cards.len());
        for card in cards.iter() {
            let height = card.lines.len().saturating_add(2) as u16;
            constraints.push(Constraint::Length(height.max(3)));
        }
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(*column_area);
        for (card_index, card) in cards.iter().enumerate() {
            let Some(area) = rows.get(card_index) else {
                continue;
            };
            let card_lines = card.lines.iter().map(render_panel_line).collect::<Vec<_>>();
            let card_panel = Paragraph::new(card_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(card.title.as_str()),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(card_panel, *area);
        }
    }
}

fn status_column_constraints(columns: usize) -> Vec<Constraint> {
    let columns = columns.max(1);
    let base = 100u16 / columns as u16;
    let remainder = 100u16 % columns as u16;
    (0..columns)
        .map(|index| {
            let extra = if (index as u16) < remainder { 1 } else { 0 };
            Constraint::Percentage(base + extra)
        })
        .collect()
}

fn assign_status_cards<'a>(cards: &'a [StatusCard], columns: usize) -> Vec<Vec<&'a StatusCard>> {
    let columns = columns.max(1);
    let mut column_cards: Vec<Vec<&StatusCard>> = vec![Vec::new(); columns];
    let mut column_heights: Vec<u16> = vec![0; columns];
    for card in cards {
        let card_height = card.lines.len().saturating_add(2) as u16;
        let (index, _) = column_heights
            .iter()
            .enumerate()
            .min_by_key(|(_, height)| *height)
            .unwrap_or((0, &0));
        column_cards[index].push(card);
        column_heights[index] = column_heights[index].saturating_add(card_height);
    }
    column_cards
}

pub fn render_panel_line(line: &MenuPanelLine) -> Line<'_> {
    let spans = line
        .spans
        .iter()
        .map(|span| Span::styled(span.text.as_str(), panel_span_style_with_palette(span)))
        .collect::<Vec<_>>();
    Line::from(spans)
}

fn panel_span_style_with_palette(span: &MenuPanelSpan) -> Style {
    let base = panel_span_style(span.style);
    if !matches!(span.style, PanelSpanStyle::Normal) {
        return base;
    }
    if let Some(color) = span.palette.as_deref().and_then(palette_color) {
        return base.fg(color);
    }
    base
}

pub fn panel_span_style(style: PanelSpanStyle) -> Style {
    match style {
        PanelSpanStyle::Normal => Style::default().fg(Color::White),
        PanelSpanStyle::Highlight => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        PanelSpanStyle::Muted => Style::default().fg(Color::DarkGray),
        PanelSpanStyle::Accent => Style::default().fg(Color::Cyan),
        PanelSpanStyle::Positive => Style::default().fg(Color::Green),
        PanelSpanStyle::Negative => Style::default().fg(Color::Red),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_style_uses_palette_color_when_known() {
        let line = MenuPanelLine {
            spans: vec![MenuPanelSpan {
                text: "~".to_string(),
                style: PanelSpanStyle::Normal,
                palette: Some("blue".to_string()),
            }],
        };

        let rendered = render_panel_line(&line);
        assert_eq!(rendered.spans[0].style.fg, Some(Color::Blue));
    }

    #[test]
    fn non_normal_style_ignores_palette_color() {
        let line = MenuPanelLine {
            spans: vec![MenuPanelSpan {
                text: "X".to_string(),
                style: PanelSpanStyle::Highlight,
                palette: Some("green".to_string()),
            }],
        };

        let rendered = render_panel_line(&line);
        assert_eq!(rendered.spans[0].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn unknown_palette_falls_back_to_base_style() {
        let line = MenuPanelLine {
            spans: vec![MenuPanelSpan {
                text: ".".to_string(),
                style: PanelSpanStyle::Normal,
                palette: Some("not_a_palette".to_string()),
            }],
        };

        let rendered = render_panel_line(&line);
        assert_eq!(rendered.spans[0].style.fg, Some(Color::White));
    }
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
