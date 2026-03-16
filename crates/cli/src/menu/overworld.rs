use std::collections::HashMap;

use engine::maps::MapFile;
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelView, PanelSpanStyle};

use super::inventory::{panel_line, panel_line_spans, panel_span, panel_span_with_palette};
use super::panels::PanelSize;

pub(super) struct OverworldDestination {
    pub(super) label: String,
    pub(super) map_id: String,
    pub(super) target_pos: (i32, i32),
    pub(super) map_pos: (i32, i32),
    pub(super) enabled: bool,
    pub(super) reason: Option<String>,
    pub(super) cost: Option<engine::maps::MapCurrencyStack>,
}

struct OverworldMapView {
    width: u32,
    height: u32,
    tiles: Vec<String>,
}

struct MapMarker {
    glyph: char,
    style: PanelSpanStyle,
}

pub(super) fn build_overworld_map_panel(
    runtime: &GameRuntime,
    panel_size: PanelSize,
    title: &str,
    allow_travel: bool,
) -> MenuPanelView {
    let Some(map) = overworld_base_map(runtime) else {
        return MenuPanelView {
            title: title.to_string(),
            lines: vec![panel_line("Overworld map unavailable.")],
        };
    };
    if allow_travel && !fast_travel_enabled(runtime) {
        return MenuPanelView {
            title: title.to_string(),
            lines: vec![
                panel_line("Fast travel unavailable."),
                panel_line("Unlock fast travel to use destinations."),
            ],
        };
    }
    let destinations = build_overworld_destinations(runtime, map);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(destinations.len().saturating_sub(1));
    let mut list_lines =
        build_destination_list_lines(runtime, &destinations, selection, panel_size.width);
    let mut view_only_line = if allow_travel {
        None
    } else {
        Some(panel_line_spans(vec![panel_span(
            "View only.",
            PanelSpanStyle::Muted,
        )]))
    };
    let mut reserved_lines = list_lines.len();
    if !list_lines.is_empty() {
        reserved_lines += 1;
    }
    if view_only_line.is_some() {
        reserved_lines += 1;
    }
    let mut map_height = panel_size.height.saturating_sub(reserved_lines as u16);
    if map_height == 0 {
        map_height = panel_size.height;
        list_lines.clear();
        view_only_line = None;
    }
    let map_view = build_overworld_map_view(map, panel_size.width, map_height);
    let markers = build_overworld_markers(runtime, map, &map_view, &destinations, selection);
    let use_color = runtime
        .content
        .rules
        .render
        .palette
        .eq_ignore_ascii_case("terminal");
    let tile_palettes = map
        .legend
        .iter()
        .filter_map(|(glyph, entry)| {
            let key = glyph.chars().next()?;
            let palette = entry
                .palette
                .as_ref()
                .filter(|palette| !palette.trim().is_empty())?
                .clone();
            Some((key, palette))
        })
        .collect::<HashMap<_, _>>();
    let mut lines = build_overworld_map_lines(&map_view, &markers, &tile_palettes, use_color);
    if map_height < panel_size.height && !list_lines.is_empty() {
        lines.push(panel_line(""));
    }
    lines.extend(list_lines);
    if let Some(line) = view_only_line {
        lines.push(line);
    }
    MenuPanelView {
        title: title.to_string(),
        lines,
    }
}

fn build_overworld_map_lines(
    view: &OverworldMapView,
    markers: &HashMap<(i32, i32), MapMarker>,
    tile_palettes: &HashMap<char, String>,
    use_color: bool,
) -> Vec<MenuPanelLine> {
    let mut lines = Vec::new();
    for y in 0..view.height as i32 {
        let row = view
            .tiles
            .get(y as usize)
            .map(|row| row.as_str())
            .unwrap_or("");
        let mut spans = Vec::new();
        for x in 0..view.width as i32 {
            if let Some(marker) = markers.get(&(x, y)) {
                spans.push(panel_span(marker.glyph.to_string(), marker.style));
                continue;
            }
            let ch = row.chars().nth(x as usize).unwrap_or(' ');
            let palette = if use_color {
                tile_palettes.get(&ch).cloned()
            } else {
                None
            };
            spans.push(panel_span_with_palette(
                ch.to_string(),
                PanelSpanStyle::Normal,
                palette,
            ));
        }
        lines.push(panel_line_spans(spans));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overworld_map_line_uses_tile_palette_in_color_mode() {
        let view = OverworldMapView {
            width: 1,
            height: 1,
            tiles: vec!["~".to_string()],
        };
        let markers = HashMap::new();
        let tile_palettes = HashMap::from([('~', "blue".to_string())]);

        let lines = build_overworld_map_lines(&view, &markers, &tile_palettes, true);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[0].spans[0].text, "~");
        assert!(matches!(lines[0].spans[0].style, PanelSpanStyle::Normal));
        assert_eq!(lines[0].spans[0].palette.as_deref(), Some("blue"));
    }

    #[test]
    fn overworld_map_line_skips_tile_palette_when_color_disabled() {
        let view = OverworldMapView {
            width: 1,
            height: 1,
            tiles: vec!["~".to_string()],
        };
        let markers = HashMap::new();
        let tile_palettes = HashMap::from([('~', "blue".to_string())]);

        let lines = build_overworld_map_lines(&view, &markers, &tile_palettes, false);

        assert_eq!(lines[0].spans[0].palette, None);
    }

    #[test]
    fn marker_style_overrides_tile_palette() {
        let view = OverworldMapView {
            width: 1,
            height: 1,
            tiles: vec!["~".to_string()],
        };
        let markers = HashMap::from([(
            (0, 0),
            MapMarker {
                glyph: 'X',
                style: PanelSpanStyle::Highlight,
            },
        )]);
        let tile_palettes = HashMap::from([('~', "blue".to_string())]);

        let lines = build_overworld_map_lines(&view, &markers, &tile_palettes, true);

        assert_eq!(lines[0].spans[0].text, "X");
        assert!(matches!(lines[0].spans[0].style, PanelSpanStyle::Highlight));
        assert_eq!(lines[0].spans[0].palette, None);
    }
}

fn build_destination_list_lines(
    runtime: &GameRuntime,
    destinations: &[OverworldDestination],
    selection: usize,
    width: u16,
) -> Vec<MenuPanelLine> {
    if destinations.is_empty() {
        return vec![panel_line_spans(vec![panel_span(
            "No destinations available.",
            PanelSpanStyle::Muted,
        )])];
    }
    let selected_index = selection.min(destinations.len().saturating_sub(1));
    let destination = &destinations[selected_index];
    let header = format!("Destination {}/{}", selected_index + 1, destinations.len());
    let mut label = destination.label.clone();
    if let Some(cost) = destination.cost.as_ref() {
        label.push_str(" ");
        label.push_str(&format_currency_amount(&runtime.content.rules, cost));
    }
    if let Some(reason) = destination.reason.as_ref() {
        label.push_str(" (");
        label.push_str(reason);
        label.push_str(")");
    }
    let header_text = tui::utils::truncate_line(&header, width as usize);
    let label_text = tui::utils::truncate_line(&label, width as usize);
    let style = if destination.enabled {
        PanelSpanStyle::Highlight
    } else {
        PanelSpanStyle::Muted
    };
    vec![
        panel_line_spans(vec![panel_span(header_text, PanelSpanStyle::Accent)]),
        panel_line_spans(vec![panel_span(label_text, style)]),
    ]
}

fn build_overworld_markers(
    runtime: &GameRuntime,
    map: &MapFile,
    view: &OverworldMapView,
    destinations: &[OverworldDestination],
    selection: usize,
) -> HashMap<(i32, i32), MapMarker> {
    let mut markers = HashMap::new();
    for (index, destination) in destinations.iter().enumerate() {
        let view_pos = map_pos_to_view_pos(map, view, destination.map_pos);
        let selected = index == selection;
        let glyph = if selected { 'X' } else { '*' };
        let style = if selected {
            PanelSpanStyle::Highlight
        } else if destination.enabled {
            PanelSpanStyle::Accent
        } else {
            PanelSpanStyle::Muted
        };
        markers.insert(view_pos, MapMarker { glyph, style });
    }

    for vehicle in &map.vehicles {
        if let Some(flags) = vehicle.requires_flags.as_ref() {
            if !flags.iter().all(|flag| runtime.has_flag(flag)) {
                continue;
            }
        }
        let vehicle_def = match runtime
            .content
            .vehicles
            .vehicles
            .iter()
            .find(|entry| entry.id == vehicle.vehicle_id)
        {
            Some(vehicle_def) => vehicle_def,
            None => continue,
        };
        if !vehicle_def.unlock_flag.trim().is_empty() && !runtime.has_flag(&vehicle_def.unlock_flag)
        {
            continue;
        }
        let vehicle_position = runtime
            .vehicle_positions
            .get(&vehicle.vehicle_id)
            .map(|entry| (entry.map_id.clone(), entry.pos));
        let map_pos = if let Some((map_id, pos)) = vehicle_position {
            if map_id != map.id {
                continue;
            }
            (pos.0, pos.1)
        } else {
            (vehicle.pos[0], vehicle.pos[1])
        };
        let glyph = vehicle_def
            .glyph
            .as_ref()
            .and_then(|glyph| glyph.chars().next())
            .unwrap_or('V');
        let view_pos = map_pos_to_view_pos(map, view, map_pos);
        markers.entry(view_pos).or_insert(MapMarker {
            glyph,
            style: PanelSpanStyle::Accent,
        });
    }

    if let Some(player_pos) = player_marker_pos(runtime, map) {
        let view_pos = map_pos_to_view_pos(map, view, player_pos);
        markers.insert(
            view_pos,
            MapMarker {
                glyph: '@',
                style: PanelSpanStyle::Accent,
            },
        );
    }
    markers
}

fn player_marker_pos(runtime: &GameRuntime, map: &MapFile) -> Option<(i32, i32)> {
    if runtime.is_overworld_map(&runtime.world.map_id) && runtime.world.map_id == map.id {
        return Some(runtime.world.position);
    }
    map.transitions
        .iter()
        .find(|transition| transition.target_map == runtime.world.map_id)
        .map(|transition| (transition.pos[0], transition.pos[1]))
}

fn build_overworld_destinations(runtime: &GameRuntime, map: &MapFile) -> Vec<OverworldDestination> {
    let mut destinations = Vec::new();
    for transition in &map.transitions {
        let target_index = match runtime.content.map_index.get(&transition.target_map) {
            Some(index) => *index,
            None => continue,
        };
        if !map_visited(runtime, &transition.target_map) {
            continue;
        }
        let target_map = match runtime.content.maps.get(target_index) {
            Some(map) => map,
            None => continue,
        };
        let label = transition
            .label
            .as_ref()
            .filter(|label| !label.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| {
                if target_map.name.trim().is_empty() {
                    transition.target_map.clone()
                } else {
                    target_map.name.clone()
                }
            });
        let mut enabled = true;
        let mut reason = None;
        if let Some(flag) = transition
            .requires_flag
            .as_ref()
            .filter(|flag| !flag.trim().is_empty())
        {
            if !runtime.has_flag(flag) {
                enabled = false;
                reason = Some("Locked".to_string());
            }
        }
        if let Some(cost) = transition.cost.as_ref() {
            let available = runtime.inventory.currency_amount(&cost.id);
            if available < cost.amount {
                enabled = false;
                reason = Some(format!(
                    "Need {}",
                    format_currency_amount(&runtime.content.rules, cost)
                ));
            }
        }
        destinations.push(OverworldDestination {
            label,
            map_id: transition.target_map.clone(),
            target_pos: (transition.target_pos[0], transition.target_pos[1]),
            map_pos: (transition.pos[0], transition.pos[1]),
            enabled,
            reason,
            cost: transition.cost.clone(),
        });
    }
    destinations
}

pub(super) fn overworld_destinations_for_runtime(
    runtime: &GameRuntime,
) -> Vec<OverworldDestination> {
    let Some(map) = overworld_base_map(runtime) else {
        return Vec::new();
    };
    build_overworld_destinations(runtime, map)
}

pub(super) fn move_overworld_selection(current: usize, count: usize, direction: i32) -> usize {
    if count == 0 {
        return 0;
    }
    if direction < 0 {
        if current == 0 {
            count.saturating_sub(1)
        } else {
            current - 1
        }
    } else if current + 1 >= count {
        0
    } else {
        current + 1
    }
}

fn overworld_base_map(runtime: &GameRuntime) -> Option<&MapFile> {
    let world = runtime
        .content
        .worlds
        .worlds
        .iter()
        .find(|world| world.id == runtime.world.world_id)?;
    let map_index = runtime.content.map_index.get(&world.overworld_map_id)?;
    runtime.content.maps.get(*map_index)
}

pub(super) fn overworld_map_available(runtime: &GameRuntime) -> bool {
    overworld_base_map(runtime).is_some()
}

fn build_overworld_map_view(
    map: &MapFile,
    target_width: u16,
    target_height: u16,
) -> OverworldMapView {
    let width = map.width.max(1);
    let height = map.height.max(1);
    let base_width = target_width.max(1);
    let base_height = target_height.max(1);
    let min_width = 6.min(base_width);
    let min_height = 4.min(base_height);
    let target_width = base_width.clamp(min_width, base_width).min(width as u16) as u32;
    let target_height = base_height
        .clamp(min_height, base_height)
        .min(height as u16) as u32;
    let mut tiles = Vec::new();
    if width == target_width && height == target_height {
        for y in 0..target_height {
            let row = map.tiles.get(y as usize).map(|row| row.as_str());
            let mut line = String::new();
            for x in 0..target_width {
                let ch = row
                    .and_then(|row| row.chars().nth(x as usize))
                    .unwrap_or(' ');
                line.push(ch);
            }
            tiles.push(line);
        }
    } else {
        for y in 0..target_height {
            let map_y = (y * height) / target_height;
            let mut line = String::new();
            for x in 0..target_width {
                let map_x = (x * width) / target_width;
                line.push(map_tile_at(map, map_x as i32, map_y as i32));
            }
            tiles.push(line);
        }
    }
    OverworldMapView {
        width: target_width,
        height: target_height,
        tiles,
    }
}

fn map_tile_at(map: &MapFile, x: i32, y: i32) -> char {
    if x < 0 || y < 0 {
        return ' ';
    }
    let row = match map.tiles.get(y as usize) {
        Some(row) => row,
        None => return ' ',
    };
    row.chars().nth(x as usize).unwrap_or(' ')
}

fn map_pos_to_view_pos(map: &MapFile, view: &OverworldMapView, pos: (i32, i32)) -> (i32, i32) {
    if map.width == 0 || map.height == 0 || view.width == 0 || view.height == 0 {
        return (0, 0);
    }
    let view_x = (pos.0.max(0) as i64 * view.width as i64 / map.width as i64) as i32;
    let view_y = (pos.1.max(0) as i64 * view.height as i64 / map.height as i64) as i32;
    (
        view_x.clamp(0, view.width.saturating_sub(1) as i32),
        view_y.clamp(0, view.height.saturating_sub(1) as i32),
    )
}

fn map_visited(runtime: &GameRuntime, map_id: &str) -> bool {
    runtime
        .map_states
        .get(map_id)
        .map(|state| state.flags.contains("visited"))
        .unwrap_or(false)
}

fn fast_travel_enabled(runtime: &GameRuntime) -> bool {
    let world = match runtime
        .content
        .worlds
        .worlds
        .iter()
        .find(|world| world.id == runtime.world.world_id)
    {
        Some(world) => world,
        None => return false,
    };
    if !world.fast_travel.enabled {
        return false;
    }
    if world.fast_travel.requires_flag.trim().is_empty() {
        return true;
    }
    runtime.has_flag(&world.fast_travel.requires_flag)
}

pub(super) fn overworld_travel_allowed(runtime: &GameRuntime) -> bool {
    super::system_enabled(runtime, Some("fast_travel")) && fast_travel_enabled(runtime)
}

fn format_currency_amount(
    rules: &engine::rules::RulesFile,
    cost: &engine::maps::MapCurrencyStack,
) -> String {
    if let Some(currency) = rules.game.currency(&cost.id) {
        if currency.symbol.trim().is_empty() {
            format!("{} {}", cost.amount, currency.name)
        } else {
            format!("{}{}", currency.symbol, cost.amount)
        }
    } else {
        format!("{} {}", cost.amount, cost.id)
    }
}
