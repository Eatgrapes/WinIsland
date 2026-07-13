pub mod anim;
pub mod input;
pub mod items;
pub mod renderer;

pub const HOVER_ROW_KEY_BASE: u64 = 10_000;

pub use anim::SwitchAnimator;
pub use input::{
    ClickResult, WidgetPreviewHit, hit_test, hover_test, widget_delete_button_hit,
    widget_grid_geom, widget_preview_hit_test,
};
pub use renderer::{DrawItemsParams, content_height, draw_items};
