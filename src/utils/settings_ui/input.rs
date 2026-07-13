use super::items::*;
use crate::core::config::{AVAILABLE_WIDGETS, WidgetKind, WidgetSlot};
use crate::ui::widget::{WidgetGridLayout, widget_corner_radius, widget_grid_layout};

pub const WIDGET_PREVIEW_H: f32 = 420.0;
pub const WIDGET_ISLAND_PANEL_H: f32 = 300.0;

#[derive(Debug, Clone, PartialEq)]
pub enum ClickResult {
    None,
    Switch(usize),
    StepperDec(usize),
    StepperInc(usize),
    FontSelect(usize),
    FontReset(usize),
    CenterLink(usize),
    SourceButton(usize),
    RowButton(usize),
    AppItem(usize),
    FolderSelect(usize),
    FolderClear(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDirection {
    Decrement,
    Increment,
}

impl ClickResult {
    pub fn item_index(&self) -> Option<usize> {
        match self {
            ClickResult::None => None,
            ClickResult::Switch(index)
            | ClickResult::StepperDec(index)
            | ClickResult::StepperInc(index)
            | ClickResult::FontSelect(index)
            | ClickResult::FontReset(index)
            | ClickResult::CenterLink(index)
            | ClickResult::SourceButton(index)
            | ClickResult::RowButton(index)
            | ClickResult::AppItem(index)
            | ClickResult::FolderSelect(index)
            | ClickResult::FolderClear(index) => Some(*index),
        }
    }

    pub fn step_direction(&self) -> Option<StepDirection> {
        match self {
            ClickResult::StepperDec(_) => Some(StepDirection::Decrement),
            ClickResult::StepperInc(_) => Some(StepDirection::Increment),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetPreviewHit {
    None,
    Source(WidgetKind),
    Slot(usize),
}

fn in_rect(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

#[derive(Debug, Clone, Copy)]
pub struct WidgetGridGeom {
    pub cap_x: f32,
    pub cap_y: f32,
    pub cap_w: f32,
    pub cap_h: f32,
    pub cap_scale: f32,
    layout: WidgetGridLayout,
}

impl WidgetGridGeom {
    pub fn slot_rect(&self, slot: usize) -> (f32, f32, f32, f32) {
        self.layout.slot_rect(slot)
    }

    pub fn footprint_rect(&self, widget: WidgetKind, slot: usize) -> (f32, f32, f32, f32) {
        self.layout.footprint_rect(widget, slot)
    }

    pub fn slot_at_point(&self, x: f32, y: f32, include_gaps: bool) -> Option<usize> {
        self.layout.slot_at_point(x, y, include_gaps)
    }
}

pub fn widget_delete_button_center(x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
    let corner_inset = widget_corner_radius(w, h) * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    (x + w - corner_inset, y + corner_inset)
}

#[allow(clippy::too_many_arguments)]
pub fn widget_delete_button_hit(
    mx: f32,
    my: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
) -> bool {
    let (cx, cy) = widget_delete_button_center(x, y, w, h);
    let radius = (7.0 * scale).max(6.0);
    (mx - cx).powi(2) + (my - cy).powi(2) <= radius.powi(2)
}

pub fn widget_source_rect(
    row_x: f32,
    source_y: f32,
    index: usize,
    kind: WidgetKind,
) -> (f32, f32, f32, f32) {
    let source_x = row_x + index as f32 * 120.0;
    let (source_w, source_h) = match kind {
        WidgetKind::Clock => (108.0, 50.0),
        WidgetKind::Calendar => (72.0, 72.0),
        WidgetKind::Settings => (50.0, 50.0),
    };
    (source_x, source_y, source_w, source_h)
}

pub fn widget_library_items(
    widget_layout: &[WidgetSlot],
    dragging: Option<WidgetKind>,
) -> Vec<WidgetKind> {
    AVAILABLE_WIDGETS
        .iter()
        .copied()
        .filter(|kind| {
            Some(*kind) != dragging
                && !widget_layout
                    .iter()
                    .any(|entry| entry.widget == Some(*kind))
        })
        .collect()
}

pub fn widget_grid_geom(
    item_y: f32,
    width: f32,
    expanded_width: f32,
    expanded_height: f32,
) -> WidgetGridGeom {
    let content_w = width - CONTENT_PADDING * 2.0;
    let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
    let preview_w = content_w - GROUP_INNER_PAD * 2.0;
    let py = item_y + (SettingsItem::WidgetPreview.height() - WIDGET_PREVIEW_H) / 2.0;

    let mut cap_w = expanded_width;
    let mut cap_h = expanded_height;
    let max_w = preview_w - 24.0;
    let max_h = WIDGET_ISLAND_PANEL_H - 56.0;
    let mut cap_scale = 1.0;
    if cap_w > max_w || cap_h > max_h {
        let scale_w = max_w / cap_w;
        let scale_h = max_h / cap_h;
        cap_scale = scale_w.min(scale_h);
        cap_w *= cap_scale;
        cap_h *= cap_scale;
    }

    let cap_x = row_x + (preview_w - cap_w) / 2.0;
    let cap_y = py + 44.0;

    let layout = widget_grid_layout(cap_x, cap_y, cap_w, cap_h, cap_scale);

    WidgetGridGeom {
        cap_x,
        cap_y,
        cap_w,
        cap_h,
        cap_scale,
        layout,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn widget_preview_hit_test(
    mx: f32,
    my: f32,
    item_y: f32,
    width: f32,
    expanded_width: f32,
    expanded_height: f32,
    widget_layout: &[WidgetSlot],
    dragging: Option<WidgetKind>,
) -> WidgetPreviewHit {
    let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
    let py = item_y + (SettingsItem::WidgetPreview.height() - WIDGET_PREVIEW_H) / 2.0;
    let library_panel_y = py + WIDGET_ISLAND_PANEL_H + 12.0;

    let source_y = library_panel_y + 32.0;
    for (idx, kind) in widget_library_items(widget_layout, dragging)
        .iter()
        .enumerate()
    {
        let (source_x, source_y, source_w, source_h) =
            widget_source_rect(row_x, source_y, idx, *kind);
        if in_rect(mx, my, source_x, source_y, source_w, source_h) {
            return WidgetPreviewHit::Source(*kind);
        }
    }

    let geom = widget_grid_geom(item_y, width, expanded_width, expanded_height);
    if let Some(slot) = geom.slot_at_point(mx, my, dragging.is_some()) {
        return WidgetPreviewHit::Slot(slot);
    }

    WidgetPreviewHit::None
}

pub fn hit_test(items: &[SettingsItem], mx: f32, my: f32, start_y: f32, width: f32) -> ClickResult {
    let mut y = start_y;
    let content_w = width - CONTENT_PADDING * 2.0;

    for (idx, item) in items.iter().enumerate() {
        match item {
            SettingsItem::RowStepper { enabled, .. } if *enabled => {
                let cy = y + ROW_HEIGHT / 2.0;
                let btn_inc_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - STEPPER_BTN_SIZE;
                let btn_dec_x = btn_inc_x - STEPPER_BTN_SIZE - 60.0;
                let btn_y = cy - STEPPER_BTN_SIZE / 2.0;
                if in_rect(mx, my, btn_dec_x, btn_y, STEPPER_BTN_SIZE, STEPPER_BTN_SIZE) {
                    return ClickResult::StepperDec(idx);
                }
                if in_rect(mx, my, btn_inc_x, btn_y, STEPPER_BTN_SIZE, STEPPER_BTN_SIZE) {
                    return ClickResult::StepperInc(idx);
                }
            }
            SettingsItem::RowSwitch { enabled, .. } if *enabled => {
                let cy = y + ROW_HEIGHT / 2.0;
                let toggle_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - TOGGLE_W;
                let toggle_y = cy - TOGGLE_H / 2.0;
                if in_rect(mx, my, toggle_x, toggle_y, TOGGLE_W, TOGGLE_H) {
                    return ClickResult::Switch(idx);
                }
            }
            SettingsItem::RowFontPicker { reset_label, .. } => {
                let cy = y + ROW_HEIGHT / 2.0;
                let sel_w: f32 = 60.0;
                let sel_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - sel_w;
                if in_rect(mx, my, sel_x, cy - 13.0, sel_w, 26.0) {
                    return ClickResult::FontSelect(idx);
                }
                if reset_label.is_some() {
                    let rst_w: f32 = 60.0;
                    let rst_x = sel_x - rst_w - 6.0;
                    if in_rect(mx, my, rst_x, cy - 13.0, rst_w, 26.0) {
                        return ClickResult::FontReset(idx);
                    }
                }
            }
            SettingsItem::RowFolderPicker {
                clear_label,
                current_path,
                enabled,
                ..
            } if *enabled => {
                let has_path = current_path.as_ref().is_some_and(|p| !p.is_empty());
                let row_h = if has_path { 64.0 } else { ROW_HEIGHT };
                let cy = y + row_h / 2.0;
                let sel_w: f32 = 60.0;
                let sel_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - sel_w;
                if in_rect(mx, my, sel_x, cy - 13.0, sel_w, 26.0) {
                    return ClickResult::FolderSelect(idx);
                }
                if clear_label.is_some() {
                    let clr_w: f32 = 60.0;
                    let clr_x = sel_x - clr_w - 6.0;
                    if in_rect(mx, my, clr_x, cy - 13.0, clr_w, 26.0) {
                        return ClickResult::FolderClear(idx);
                    }
                }
            }
            SettingsItem::RowSourceSelect { enabled, .. } if *enabled => {
                let cy = y + ROW_HEIGHT / 2.0;
                let btn_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0;
                if in_rect(mx, my, btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H) {
                    return ClickResult::SourceButton(idx);
                }
            }
            SettingsItem::RowButton { enabled, .. } if *enabled => {
                let cy = y + ROW_HEIGHT / 2.0;
                let btn_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                let btn_y = cy - POPUP_BTN_H / 2.0;
                if in_rect(mx, my, btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H) {
                    return ClickResult::RowButton(idx);
                }
            }
            SettingsItem::RowAppItem { enabled, .. }
                if *enabled && in_rect(mx, my, CONTENT_PADDING, y, content_w, ROW_HEIGHT) =>
            {
                return ClickResult::AppItem(idx);
            }
            SettingsItem::RowLabel { .. } => {}
            SettingsItem::CenterLink { .. }
                if mx >= width / 2.0 - 100.0
                    && mx <= width / 2.0 + 100.0
                    && my >= y
                    && my <= y + 40.0 =>
            {
                return ClickResult::CenterLink(idx);
            }
            _ => {}
        }
        y += item.height();
    }
    ClickResult::None
}

pub fn hover_test(items: &[SettingsItem], mx: f32, my: f32, start_y: f32, width: f32) -> bool {
    hit_test(items, mx, my, start_y, width) != ClickResult::None
}
