use std::io::{self, ErrorKind};

use crossterm::event::{self, Event, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::Stdout;

use crate::input::{is_actionable_key, Action, InputBindings};
use crate::session::TuiSession;
use crate::ui::DialogUiFile;
use crate::utils::{centered_rect, truncate_line, wrap_text};

pub struct ChoiceView {
    pub label: String,
    pub show_next: bool,
}

pub fn show_dialog(
    session: &mut TuiSession,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
) -> io::Result<()> {
    let width = dialog_inner_width(session, dialog_ui);
    let lines = wrap_text(text, width);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while let Some(page) = pages.pop() {
        draw_dialog(session.terminal_mut(), dialog_ui, speaker, &page, None)?;
        wait_for_continue(session, bindings, |frame| {
            draw_dialog_overlay(frame, dialog_ui, speaker, &page);
        })?;
    }

    Ok(())
}

pub fn show_dialog_with_choices(
    session: &mut TuiSession,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    text: &str,
    choices: &[ChoiceView],
) -> io::Result<Option<usize>> {
    let width = dialog_inner_width(session, dialog_ui);
    let lines = wrap_text(text, width);
    let mut pages = paginate_lines(lines, dialog_ui, speaker);

    while pages.len() > 1 {
        if let Some(page) = pages.pop() {
            draw_dialog(session.terminal_mut(), dialog_ui, speaker, &page, None)?;
            wait_for_continue(session, bindings, |frame| {
                draw_dialog_overlay(frame, dialog_ui, speaker, &page);
            })?;
        }
    }

    let page = pages.pop().unwrap_or_default();
    choose_dialog_option(session, dialog_ui, bindings, speaker, &page, choices)
}

pub fn draw_dialog(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    dialog_ui: &DialogUiFile,
    speaker: &str,
    lines: &[String],
    choices: Option<(usize, &[ChoiceView])>,
) -> io::Result<()> {
    terminal
        .draw(|frame| {
            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
            if let Some((selected, choices)) = choices {
                draw_choice_box(frame, choices, selected);
            }
        })
        .map(|_| ())
}

pub fn draw_dialog_overlay(
    frame: &mut Frame,
    dialog_ui: &DialogUiFile,
    speaker: &str,
    lines: &[String],
) {
    let area = dialog_area(frame.size(), dialog_ui);
    let inner_width = area.width.saturating_sub(2) as usize;
    let mut content = Vec::new();

    frame.render_widget(Clear, area);

    if dialog_ui.show_speaker && !speaker.is_empty() {
        content.push(Line::from(Span::styled(
            speaker,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
    }

    for line in lines {
        content.push(Line::from(Span::raw(truncate_line(line, inner_width))));
    }

    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);

    if !dialog_ui.continue_marker.is_empty() {
        let marker_area = Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(2),
            1,
        );
        let marker = Paragraph::new(dialog_ui.continue_marker.as_str())
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: false });
        frame.render_widget(marker, marker_area);
    }
}

pub fn draw_centered_dialog_overlay(frame: &mut Frame, dialog_ui: &DialogUiFile, lines: &[String]) {
    let area = centered_dialog_area(frame.size(), dialog_ui);
    let inner_width = area.width.saturating_sub(2) as usize;
    let mut content = Vec::new();

    frame.render_widget(Clear, area);

    for line in lines {
        content.push(Line::from(Span::raw(truncate_line(line, inner_width))));
    }

    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);

    if !dialog_ui.continue_marker.is_empty() {
        let marker_area = Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(2),
            1,
        );
        let marker = Paragraph::new(dialog_ui.continue_marker.as_str())
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: false });
        frame.render_widget(marker, marker_area);
    }
}

pub fn wait_for_continue<F>(
    session: &mut TuiSession,
    bindings: &InputBindings,
    draw_background: F,
) -> io::Result<()>
where
    F: Fn(&mut Frame),
{
    loop {
        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                    return Ok(());
                }
                if action == Action::Quit {
                    if confirm_quit(session, &draw_background)? {
                        return Err(io::Error::new(ErrorKind::Interrupted, "quit"));
                    }
                }
            }
        }
    }
}

pub fn choose_dialog_option(
    session: &mut TuiSession,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    speaker: &str,
    lines: &[String],
    choices: &[ChoiceView],
) -> io::Result<Option<usize>> {
    let mut selected = 0usize;
    loop {
        draw_dialog(
            session.terminal_mut(),
            dialog_ui,
            speaker,
            lines,
            Some((selected, choices)),
        )?;
        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    Action::MoveDown => {
                        if selected + 1 < choices.len() {
                            selected += 1;
                        }
                    }
                    Action::Confirm => return Ok(Some(selected)),
                    Action::Cancel | Action::Menu => return Ok(None),
                    Action::Quit => {
                        if confirm_quit(session, |frame| {
                            draw_dialog_overlay(frame, dialog_ui, speaker, lines);
                            draw_choice_box(frame, choices, selected);
                        })? {
                            return Err(io::Error::new(ErrorKind::Interrupted, "quit"));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn confirm_quit<F>(session: &mut TuiSession, draw_background: F) -> io::Result<bool>
where
    F: Fn(&mut Frame),
{
    loop {
        session.terminal_mut().draw(|frame| {
            draw_background(frame);
            let area = centered_rect(frame.size(), 40, 3);
            frame.render_widget(Clear, area);
            let content = vec![Line::from(Span::raw("Quit the game? (Y/N)"))];
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
                KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                KeyCode::Char('n') | KeyCode::Char('N') => return Ok(false),
                _ => return Ok(false),
            }
        }
    }
}

pub fn prompt_text(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
    max_len: usize,
) -> io::Result<Option<String>> {
    let mut value = default.to_string();
    loop {
        session.terminal_mut().draw(|frame| {
            draw_text_prompt_frame(frame, title, prompt, &value, max_len);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            match key.code {
                KeyCode::Enter => {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        return Ok(Some(default.to_string()));
                    }
                    return Ok(Some(trimmed.to_string()));
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Backspace => {
                    value.pop();
                }
                KeyCode::Char(ch) => {
                    if value.chars().count() < max_len {
                        value.push(ch);
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn prompt_choice(
    session: &mut TuiSession,
    bindings: &InputBindings,
    title: &str,
    prompt: &str,
    options: &[String],
    mut selected: usize,
) -> io::Result<Option<usize>> {
    if options.is_empty() {
        return Ok(None);
    }
    if selected >= options.len() {
        selected = 0;
    }
    loop {
        session.terminal_mut().draw(|frame| {
            draw_choice_prompt_frame(frame, title, prompt, options, selected);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            if let Some(action) = bindings.action_for(key.code) {
                match action {
                    Action::MoveUp => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    Action::MoveDown => {
                        if selected + 1 < options.len() {
                            selected += 1;
                        }
                    }
                    Action::Confirm => return Ok(Some(selected)),
                    Action::Cancel | Action::Menu => return Ok(None),
                    Action::Quit => return Ok(None),
                    _ => {}
                }
            }
        }
    }
}

pub fn draw_text_prompt_frame(
    frame: &mut Frame,
    title: &str,
    prompt: &str,
    value: &str,
    max_len: usize,
) {
    let content = vec![
        Line::from(Span::raw(prompt)),
        Line::from(Span::styled(value, Style::default().fg(Color::Yellow))),
        Line::from(Span::raw(format!("{}/{}", value.chars().count(), max_len))),
    ];
    let width = content
        .iter()
        .map(|line| line.width() as u16)
        .max()
        .unwrap_or(20)
        .saturating_add(4)
        .max(30);
    let height = content.len() as u16 + 2;
    let area = centered_rect(frame.size(), width, height);
    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

pub fn draw_choice_prompt_frame(
    frame: &mut Frame,
    title: &str,
    prompt: &str,
    options: &[String],
    selected: usize,
) {
    let max_option = options
        .iter()
        .map(|item| item.chars().count())
        .max()
        .unwrap_or(8);
    let header = Line::from(Span::raw(prompt));
    let list = options
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let mut style = Style::default();
            if index == selected {
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }
            Line::from(Span::styled(label.as_str(), style))
        })
        .collect::<Vec<_>>();
    let mut lines = Vec::with_capacity(list.len() + 1);
    lines.push(header);
    lines.extend(list);

    let width = (max_option as u16).saturating_add(6).max(26);
    let height = (lines.len() as u16).saturating_add(2);
    let area = centered_rect(frame.size(), width, height);
    frame.render_widget(Clear, area);
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

pub fn draw_choice_box(frame: &mut Frame, choices: &[ChoiceView], selected: usize) {
    if choices.is_empty() {
        return;
    }

    let max_len = choices
        .iter()
        .map(|choice| choice.label.chars().count())
        .max()
        .unwrap_or(0);
    let width = (max_len as u16).saturating_add(2).max(12);
    let height = (choices.len() as u16).saturating_add(2);
    let area = centered_rect(frame.size(), width, height);

    frame.render_widget(Clear, area);

    let lines = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            let text = choice.label.as_str();
            let style = if index == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(text, style))
        })
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

pub fn paginate_lines(
    mut lines: Vec<String>,
    dialog_ui: &DialogUiFile,
    speaker: &str,
) -> Vec<Vec<String>> {
    let mut pages = Vec::new();
    let available_lines = dialog_ui.height.saturating_sub(2) as usize;
    let speaker_offset = if dialog_ui.show_speaker && !speaker.is_empty() {
        1
    } else {
        0
    };
    let page_size = available_lines.saturating_sub(speaker_offset).max(1);

    while !lines.is_empty() {
        let count = page_size.min(lines.len());
        let page = lines.drain(0..count).collect::<Vec<_>>();
        pages.insert(0, page);
    }

    pages
}

pub fn dialog_area(area: Rect, dialog_ui: &DialogUiFile) -> Rect {
    let height = dialog_ui.height.min(area.height);
    match dialog_ui.position.as_str() {
        "top" => Rect::new(area.x, area.y, area.width, height),
        _ => Rect::new(
            area.x,
            area.y + area.height.saturating_sub(height),
            area.width,
            height,
        ),
    }
}

pub fn centered_dialog_area(area: Rect, dialog_ui: &DialogUiFile) -> Rect {
    let height = dialog_ui.height.min(area.height).max(3);
    let width = centered_dialog_width_for_area(area);
    centered_rect(area, width, height)
}

pub fn centered_dialog_width(session: &TuiSession) -> usize {
    let area = session.terminal().size().unwrap_or_default();
    let width = centered_dialog_width_for_area(area);
    width.saturating_sub(2).max(1) as usize
}

pub fn centered_dialog_width_for_area(area: Rect) -> u16 {
    let width = area.width.saturating_sub(10).max(20);
    width.min(60).min(area.width.saturating_sub(2))
}

pub fn dialog_inner_width(session: &TuiSession, dialog_ui: &DialogUiFile) -> usize {
    let area = session.terminal().size().unwrap_or_default();
    let dialog = dialog_area(area, dialog_ui);
    dialog.width.saturating_sub(2).max(1) as usize
}
