use skia_safe::Color;

pub const COLOR_CARD_HIGHLIGHT: Color = Color::from_rgb(63, 63, 66);
pub const COLOR_ACCENT: Color = Color::from_rgb(10, 132, 255);
pub const COLOR_TEXT_PRI: Color = Color::WHITE;
pub const COLOR_TEXT_SEC: Color = Color::from_rgb(142, 142, 147);
pub const COLOR_DANGER: Color = Color::from_rgb(255, 69, 58);
pub const COLOR_DISABLED: Color = Color::from_rgb(60, 60, 60);

pub const COLOR_WIN_BG: Color = Color::from_rgb(30, 30, 30);
pub const COLOR_SIDEBAR_BG: Color = Color::from_rgb(42, 42, 42);
pub const COLOR_GROUP_BG: Color = Color::from_rgb(44, 44, 46);
pub const COLOR_TOGGLE_ON: Color = Color::from_rgb(48, 209, 88);
pub const COLOR_TOGGLE_OFF: Color = Color::from_rgb(57, 57, 61);

pub fn color_sidebar_sel() -> Color {
    Color::from_argb(50, 10, 132, 255)
}

pub fn color_sidebar_hover() -> Color {
    Color::from_argb(8, 255, 255, 255)
}

pub fn color_separator() -> Color {
    Color::from_argb(26, 255, 255, 255)
}

pub fn get_island_border_weights(_cx: i32, _cy: i32, _w: f32, _h: f32) -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.0]
}

pub struct SettingsTheme {
    pub win_bg: Color,
    pub sidebar_bg: Color,
    pub group_bg: Color,
    pub card_highlight: Color,
    pub text_pri: Color,
    pub text_sec: Color,
    pub disabled: Color,
    pub accent: Color,
    pub danger: Color,
    pub toggle_on: Color,
    pub toggle_off: Color,
    pub sidebar_sel: Color,
    pub sidebar_hover: Color,
    pub separator: Color,
    pub popup_bg: Color,
    pub popup_border: Color,
    pub hover_row: Color,
}

pub fn dark_settings_theme() -> SettingsTheme {
    SettingsTheme {
        win_bg: COLOR_WIN_BG,
        sidebar_bg: COLOR_SIDEBAR_BG,
        group_bg: COLOR_GROUP_BG,
        card_highlight: COLOR_CARD_HIGHLIGHT,
        text_pri: COLOR_TEXT_PRI,
        text_sec: COLOR_TEXT_SEC,
        disabled: COLOR_DISABLED,
        accent: COLOR_ACCENT,
        danger: COLOR_DANGER,
        toggle_on: COLOR_TOGGLE_ON,
        toggle_off: COLOR_TOGGLE_OFF,
        sidebar_sel: color_sidebar_sel(),
        sidebar_hover: color_sidebar_hover(),
        separator: color_separator(),
        popup_bg: Color::from_rgb(50, 50, 52),
        popup_border: Color::from_argb(40, 255, 255, 255),
        hover_row: Color::from_argb(15, 255, 255, 255),
    }
}

pub fn light_settings_theme() -> SettingsTheme {
    SettingsTheme {
        win_bg: Color::from_rgb(242, 242, 247),
        sidebar_bg: Color::from_rgb(232, 232, 237),
        group_bg: Color::from_rgb(255, 255, 255),
        card_highlight: Color::from_rgb(218, 218, 223),
        text_pri: Color::from_rgb(0, 0, 0),
        text_sec: Color::from_rgb(99, 99, 102),
        disabled: Color::from_rgb(194, 194, 199),
        accent: Color::from_rgb(0, 122, 255),
        danger: Color::from_rgb(255, 59, 48),
        toggle_on: Color::from_rgb(52, 199, 89),
        toggle_off: Color::from_rgb(178, 178, 183),
        sidebar_sel: Color::from_argb(50, 0, 122, 255),
        sidebar_hover: Color::from_argb(8, 0, 0, 0),
        separator: Color::from_argb(26, 0, 0, 0),
        popup_bg: Color::from_rgb(255, 255, 255),
        popup_border: Color::from_argb(40, 0, 0, 0),
        hover_row: Color::from_argb(12, 0, 0, 0),
    }
}
