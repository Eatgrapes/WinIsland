use serde::{Deserialize, Serialize};
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_AUTHOR: &str = "Eatgrapes";
pub const APP_HOMEPAGE: &str = "https://github.com/Eatgrapes/WinIsland";
pub const WINDOW_TITLE: &str = "WinIsland";
pub const TOP_OFFSET: i32 = 10;
pub const PADDING: f32 = 80.0;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(from = "String", into = "String")]
#[derive(Default)]
pub enum DockPosition {
    #[default]
    TopCenter,
    TopLeft,
    TopRight,
    BottomCenter,
    BottomLeft,
    BottomRight,
}

impl DockPosition {
    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::BottomCenter | Self::BottomLeft | Self::BottomRight
        )
    }

    pub fn is_left(&self) -> bool {
        matches!(self, Self::TopLeft | Self::BottomLeft)
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TopCenter => "top_center",
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomCenter => "bottom_center",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
        }
    }
}

impl std::fmt::Display for DockPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for DockPosition {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "top_center" => Ok(Self::TopCenter),
            "top_left" => Ok(Self::TopLeft),
            "top_right" => Ok(Self::TopRight),
            "bottom_center" => Ok(Self::BottomCenter),
            "bottom_left" => Ok(Self::BottomLeft),
            "bottom_right" => Ok(Self::BottomRight),
            _ => Err(()),
        }
    }
}

impl From<String> for DockPosition {
    fn from(value: String) -> Self {
        value.parse().unwrap_or_default()
    }
}

impl From<DockPosition> for String {
    fn from(value: DockPosition) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    Clock,
    Calendar,
}

impl WidgetKind {
    pub const fn span(&self) -> (usize, usize) {
        match self {
            WidgetKind::Clock => (1, 1),
            WidgetKind::Calendar => (1, 2),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WidgetSlot {
    pub slot: usize,
    #[serde(default, deserialize_with = "deserialize_widget_kind")]
    pub widget: Option<WidgetKind>,
}

fn deserialize_widget_kind<'de, D>(deserializer: D) -> Result<Option<WidgetKind>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.and_then(|s| match s.as_str() {
        "clock" => Some(WidgetKind::Clock),
        "calendar" => Some(WidgetKind::Calendar),
        _ => None,
    }))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub global_scale: f32,
    pub base_width: f32,
    pub base_height: f32,
    pub expanded_width: f32,
    pub expanded_height: f32,
    pub adaptive_border: bool,
    pub motion_blur: bool,
    #[serde(default = "default_island_style")]
    pub island_style: String,
    pub smtc_enabled: bool,
    pub smtc_apps: Vec<String>,
    #[serde(default = "default_smtc_known_apps")]
    pub smtc_known_apps: Vec<String>,
    #[serde(default = "default_show_lyrics")]
    pub show_lyrics: bool,
    #[serde(default = "default_lyrics_local_dir")]
    pub lyrics_local_dir: Option<String>,
    #[serde(default = "default_custom_font")]
    pub custom_font_path: Option<String>,
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,
    #[serde(default = "default_auto_hide")]
    pub auto_hide: bool,
    #[serde(default = "default_auto_hide_delay")]
    pub auto_hide_delay: f32,
    #[serde(default = "default_check_for_updates")]
    pub check_for_updates: bool,
    #[serde(default = "default_update_check_interval")]
    pub update_check_interval: f32,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_lyrics_source")]
    pub lyrics_source: String,
    #[serde(default = "default_lyrics_fallback")]
    pub lyrics_fallback: bool,
    #[serde(default = "default_lyrics_delay")]
    pub lyrics_delay: f64,
    #[serde(default = "default_lyrics_scroll")]
    pub lyrics_scroll: bool,
    #[serde(default = "default_lyrics_scroll_max_width")]
    pub lyrics_scroll_max_width: f32,
    #[serde(default = "default_position_x_offset")]
    pub position_x_offset: i32,
    #[serde(default = "default_position_y_offset")]
    pub position_y_offset: i32,
    #[serde(default = "default_dock_position")]
    pub dock_position: DockPosition,
    #[serde(default = "default_monitor_index")]
    pub monitor_index: i32,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_settings_theme")]
    pub settings_theme: String,
    #[serde(default = "default_mini_cover_shape")]
    pub mini_cover_shape: String,
    #[serde(default = "default_expanded_cover_shape")]
    pub expanded_cover_shape: String,
    #[serde(default = "default_cover_rotate")]
    pub cover_rotate: bool,
    #[serde(default = "default_update_channel")]
    pub update_channel: String,
    #[serde(default = "default_right_click_drag")]
    pub right_click_drag: bool,
    #[serde(default = "default_widget_layout")]
    pub widget_layout: Vec<WidgetSlot>,
}

fn default_right_click_drag() -> bool {
    false
}

fn default_island_style() -> String {
    "default".to_string()
}

fn default_show_lyrics() -> bool {
    true
}

fn default_smtc_known_apps() -> Vec<String> {
    Vec::new()
}

fn default_custom_font() -> Option<String> {
    None
}

fn default_lyrics_local_dir() -> Option<String> {
    None
}

fn default_auto_start() -> bool {
    false
}

fn default_auto_hide() -> bool {
    false
}

fn default_auto_hide_delay() -> f32 {
    5.0
}

fn default_check_for_updates() -> bool {
    true
}

fn default_update_check_interval() -> f32 {
    4.0
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_lyrics_source() -> String {
    "163".to_string()
}

fn default_lyrics_fallback() -> bool {
    true
}

fn default_lyrics_delay() -> f64 {
    0.0
}

fn default_lyrics_scroll() -> bool {
    false
}

fn default_lyrics_scroll_max_width() -> f32 {
    300.0
}

fn default_position_x_offset() -> i32 {
    0
}

fn default_position_y_offset() -> i32 {
    0
}

fn default_dock_position() -> DockPosition {
    DockPosition::TopCenter
}

fn default_monitor_index() -> i32 {
    0
}

fn default_font_size() -> f32 {
    0.0
}

fn default_settings_theme() -> String {
    "system".to_string()
}

fn default_mini_cover_shape() -> String {
    "square".to_string()
}

fn default_expanded_cover_shape() -> String {
    "square".to_string()
}

fn default_cover_rotate() -> bool {
    false
}

fn default_update_channel() -> String {
    "stable".to_string()
}

pub const WIDGET_GRID_COLS: usize = 3;
pub const WIDGET_GRID_ROWS: usize = 3;
pub const WIDGET_GRID_SLOTS: usize = WIDGET_GRID_COLS * WIDGET_GRID_ROWS;
pub const AVAILABLE_WIDGETS: [WidgetKind; 2] = [WidgetKind::Clock, WidgetKind::Calendar];

pub fn widget_footprint(widget: WidgetKind, anchor_slot: usize) -> Vec<usize> {
    let (cols, rows) = widget.span();
    let anchor_col = (anchor_slot % WIDGET_GRID_COLS).min(WIDGET_GRID_COLS - cols);
    let anchor_row = (anchor_slot / WIDGET_GRID_COLS).min(WIDGET_GRID_ROWS - rows);
    let mut cells = Vec::with_capacity(cols * rows);
    for dr in 0..rows {
        for dc in 0..cols {
            cells.push((anchor_row + dr) * WIDGET_GRID_COLS + (anchor_col + dc));
        }
    }
    cells
}

pub fn widget_anchor_slot(widget: WidgetKind, target_slot: usize) -> usize {
    *widget_footprint(widget, target_slot)
        .first()
        .unwrap_or(&target_slot)
}

pub fn widget_covering_slot(
    layout: &[WidgetSlot],
    target_slot: usize,
) -> Option<(usize, WidgetKind)> {
    layout.iter().find_map(|entry| {
        let widget = entry.widget?;
        widget_footprint(widget, entry.slot)
            .contains(&target_slot)
            .then_some((entry.slot, widget))
    })
}

pub fn default_widget_layout() -> Vec<WidgetSlot> {
    (0..WIDGET_GRID_SLOTS)
        .map(|slot| WidgetSlot { slot, widget: None })
        .collect()
}

fn clear_cells(layout: &mut [WidgetSlot], cells: &[usize]) {
    let occupants: Vec<usize> = layout
        .iter()
        .filter_map(|entry| entry.widget.map(|w| (entry.slot, w)))
        .filter(|(anchor, w)| {
            widget_footprint(*w, *anchor)
                .iter()
                .any(|cell| cells.contains(cell))
        })
        .map(|(anchor, _)| anchor)
        .collect();
    for anchor in occupants {
        if let Some(entry) = layout.iter_mut().find(|entry| entry.slot == anchor) {
            entry.widget = None;
        }
    }
}

pub fn place_widget_in_layout(
    layout: &mut Vec<WidgetSlot>,
    widget: WidgetKind,
    target_slot: usize,
) {
    let anchor = widget_anchor_slot(widget, target_slot);
    let slot_count = anchor.max(WIDGET_GRID_SLOTS - 1) + 1;
    for slot in 0..slot_count {
        if !layout.iter().any(|entry| entry.slot == slot) {
            layout.push(WidgetSlot { slot, widget: None });
        }
    }
    layout.sort_by_key(|entry| entry.slot);
    for entry in layout.iter_mut() {
        if entry.widget == Some(widget) {
            entry.widget = None;
        }
    }
    clear_cells(layout, &widget_footprint(widget, anchor));
    if let Some(entry) = layout.iter_mut().find(|entry| entry.slot == anchor) {
        entry.widget = Some(widget);
    }
}

pub fn clear_widget_slot(layout: &mut [WidgetSlot], target_slot: usize) {
    clear_cells(layout, &[target_slot]);
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            global_scale: 1.0,
            base_width: 120.0,
            base_height: 27.0,
            expanded_width: 360.0,
            expanded_height: 200.0,
            adaptive_border: false,
            motion_blur: true,
            island_style: "default".to_string(),
            smtc_enabled: true,
            smtc_apps: Vec::new(),
            smtc_known_apps: Vec::new(),
            show_lyrics: true,
            lyrics_local_dir: None,
            custom_font_path: None,
            auto_start: false,
            auto_hide: false,
            auto_hide_delay: 5.0,
            check_for_updates: true,
            update_check_interval: 4.0,
            language: "auto".to_string(),
            lyrics_source: "163".to_string(),
            lyrics_fallback: true,
            lyrics_delay: 0.0,
            lyrics_scroll: false,
            lyrics_scroll_max_width: 300.0,
            position_x_offset: 0,
            position_y_offset: 0,
            dock_position: DockPosition::TopCenter,
            monitor_index: 0,
            font_size: 0.0,
            settings_theme: "system".to_string(),
            mini_cover_shape: "square".to_string(),
            expanded_cover_shape: "square".to_string(),
            cover_rotate: false,
            update_channel: "stable".to_string(),
            right_click_drag: false,
            widget_layout: default_widget_layout(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_empty_widget_slots() {
        let config = AppConfig::default();

        assert_eq!(config.widget_layout.len(), WIDGET_GRID_SLOTS);
        assert!(
            config.widget_layout.iter().all(|s| s.widget.is_none()),
            "default layout must ship with no placeholder widgets"
        );
        for (i, entry) in config.widget_layout.iter().enumerate() {
            assert_eq!(entry.slot, i);
        }
    }

    #[test]
    fn widgets_use_their_expected_footprints() {
        assert_eq!(WidgetKind::Clock.span(), (1, 1));
        assert_eq!(WidgetKind::Calendar.span(), (1, 2));
        assert_eq!(widget_footprint(WidgetKind::Clock, 0), vec![0]);
        assert_eq!(widget_footprint(WidgetKind::Clock, 8), vec![8]);
        assert_eq!(widget_anchor_slot(WidgetKind::Clock, 8), 8);
        assert_eq!(widget_footprint(WidgetKind::Calendar, 8), vec![5, 8]);
        assert_eq!(widget_anchor_slot(WidgetKind::Calendar, 8), 5);
    }

    #[test]
    fn missing_widget_layout_deserializes_to_default_slots() {
        let config: AppConfig = toml::from_str(
            r#"
global_scale = 1.0
base_width = 120.0
base_height = 27.0
expanded_width = 360.0
expanded_height = 200.0
adaptive_border = false
motion_blur = true
smtc_enabled = true
smtc_apps = []
"#,
        )
        .unwrap();

        assert_eq!(config.widget_layout, default_widget_layout());
    }

    #[test]
    fn place_widget_in_layout_moves_existing_widget_to_target_slot() {
        let mut layout = default_widget_layout();
        place_widget_in_layout(&mut layout, WidgetKind::Clock, 0);

        place_widget_in_layout(&mut layout, WidgetKind::Clock, 3);

        assert_eq!(layout[0].widget, None);
        assert_eq!(layout[3].widget, Some(WidgetKind::Clock));
        assert_eq!(
            layout
                .iter()
                .filter(|e| e.widget == Some(WidgetKind::Clock))
                .count(),
            1
        );
    }

    #[test]
    fn place_widget_in_layout_creates_missing_slot() {
        let mut layout: Vec<WidgetSlot> = Vec::new();

        place_widget_in_layout(&mut layout, WidgetKind::Clock, 3);

        assert_eq!(layout.len(), WIDGET_GRID_SLOTS);
        assert_eq!(layout[3].widget, Some(WidgetKind::Clock));
        for (i, entry) in layout.iter().enumerate() {
            assert_eq!(entry.slot, i);
        }
    }

    #[test]
    fn clear_widget_slot_removes_widget_covering_target_slot() {
        let mut layout = default_widget_layout();
        place_widget_in_layout(&mut layout, WidgetKind::Clock, 0);

        clear_widget_slot(&mut layout, 0);

        assert!(layout.iter().all(|e| e.widget.is_none()));
    }

    #[test]
    fn widget_covering_slot_finds_multi_cell_widgets() {
        let mut layout = default_widget_layout();
        place_widget_in_layout(&mut layout, WidgetKind::Calendar, 1);

        assert_eq!(
            widget_covering_slot(&layout, 1),
            Some((1, WidgetKind::Calendar))
        );
        assert_eq!(
            widget_covering_slot(&layout, 4),
            Some((1, WidgetKind::Calendar))
        );
        assert_eq!(widget_covering_slot(&layout, 0), None);
    }

    #[test]
    fn removed_widget_kinds_deserialize_to_empty_slot() {
        let slot: WidgetSlot = toml::from_str("slot = 0\nwidget = \"status\"\n").unwrap();
        assert_eq!(slot.widget, None);
        let slot: WidgetSlot = toml::from_str("slot = 1\nwidget = \"clock\"\n").unwrap();
        assert_eq!(slot.widget, Some(WidgetKind::Clock));
        let slot: WidgetSlot = toml::from_str("slot = 2\nwidget = \"calendar\"\n").unwrap();
        assert_eq!(slot.widget, Some(WidgetKind::Calendar));
    }
}
