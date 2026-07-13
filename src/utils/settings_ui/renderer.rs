mod controls;
mod items;
mod widget_preview;

use skia_safe::Canvas;

use crate::core::config::{WidgetKind, WidgetSlot};
use crate::utils::anim::AnimPool;
use crate::utils::color::SettingsTheme;

use super::anim::SwitchAnimator;
use super::items::SettingsItem;

pub use items::{content_height, draw_items};

pub struct DrawItemsParams<'a> {
    pub canvas: &'a Canvas,
    pub items: &'a [SettingsItem],
    pub start_y: f32,
    pub width: f32,
    pub anims: &'a SwitchAnimator,
    pub hover_anims: &'a AnimPool,
    pub theme: &'a SettingsTheme,
    pub visible_min_y: f32,
    pub visible_max_y: f32,
    pub island_style: &'a str,
    pub adaptive_border: bool,
    pub expanded_width: f32,
    pub expanded_height: f32,
    pub widget_layout: &'a [WidgetSlot],
    pub widget_dragging: Option<WidgetKind>,
    pub widget_drag_hover_slot: Option<usize>,
    pub widget_preview_hover_slot: Option<usize>,
}
