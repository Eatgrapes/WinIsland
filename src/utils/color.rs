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
