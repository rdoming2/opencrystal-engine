use ratatui::layout::Rect;

pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }
        if current.len() + word.len() + 1 > width {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub fn truncate_line(line: &str, width: usize) -> String {
    if line.len() <= width {
        return line.to_string();
    }
    line.chars().take(width).collect()
}

pub fn palette_style(use_color: bool, palette: Option<&str>) -> ratatui::style::Style {
    if !use_color {
        return ratatui::style::Style::default();
    }
    match palette.and_then(palette_color) {
        Some(color) => ratatui::style::Style::default().fg(color),
        None => ratatui::style::Style::default(),
    }
}

pub fn palette_color(name: &str) -> Option<ratatui::style::Color> {
    let key = name.trim().to_ascii_lowercase();
    match key.as_str() {
        "black" => Some(ratatui::style::Color::Black),
        "red" => Some(ratatui::style::Color::Red),
        "green" => Some(ratatui::style::Color::Green),
        "yellow" => Some(ratatui::style::Color::Yellow),
        "blue" => Some(ratatui::style::Color::Blue),
        "magenta" => Some(ratatui::style::Color::Magenta),
        "cyan" => Some(ratatui::style::Color::Cyan),
        "white" => Some(ratatui::style::Color::White),
        "gray" | "grey" => Some(ratatui::style::Color::Gray),
        "dark_gray" | "dark_grey" => Some(ratatui::style::Color::DarkGray),
        "bright_black" | "light_black" => Some(ratatui::style::Color::DarkGray),
        "bright_red" | "light_red" => Some(ratatui::style::Color::LightRed),
        "bright_green" | "light_green" => Some(ratatui::style::Color::LightGreen),
        "bright_yellow" | "light_yellow" => Some(ratatui::style::Color::LightYellow),
        "bright_blue" | "light_blue" => Some(ratatui::style::Color::LightBlue),
        "bright_magenta" | "light_magenta" => Some(ratatui::style::Color::LightMagenta),
        "bright_cyan" | "light_cyan" => Some(ratatui::style::Color::LightCyan),
        "bright_white" | "light_white" => Some(ratatui::style::Color::White),
        "bright_gray" | "bright_grey" => Some(ratatui::style::Color::Gray),
        _ => None,
    }
}

pub fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}
