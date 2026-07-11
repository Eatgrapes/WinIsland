use super::items::*;
use crate::core::config::{WIDGET_GRID_SLOTS, WidgetKind};

/// Widgets are laid out in a fixed 3x3 grid.
pub const WIDGET_GRID_COLS: usize = 3;
pub const WIDGET_GRID_ROWS: usize = 3;

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
pub enum WidgetPreviewHit {
    None,
    Source(WidgetKind),
    Slot(usize),
}

fn in_rect(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

/// Geometry of the island preview and its 3x3 placement grid, shared by the
/// renderer and hit tester so drawn slots and click targets stay aligned.
#[derive(Debug, Clone, Copy)]
pub struct WidgetGridGeom {
    pub cap_x: f32,
    pub cap_y: f32,
    pub cap_w: f32,
    pub cap_h: f32,
    pub cap_scale: f32,
    slot_w: f32,
    slot_h: f32,
    gap: f32,
    grid_x: f32,
    grid_y: f32,
}

impl WidgetGridGeom {
    pub fn slot_rect(&self, slot: usize) -> (f32, f32, f32, f32) {
        let col = (slot % WIDGET_GRID_COLS) as f32;
        let row = (slot / WIDGET_GRID_COLS) as f32;
        let x = self.grid_x + col * (self.slot_w + self.gap);
        let y = self.grid_y + row * (self.slot_h + self.gap);
        (x, y, self.slot_w, self.slot_h)
    }
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

    let inset = 12.0 * cap_scale;
    let gap = 7.0 * cap_scale;
    let inner_w = cap_w - inset * 2.0;
    let inner_h = cap_h - inset * 2.0;
    let slot_w = (inner_w - gap * (WIDGET_GRID_COLS as f32 - 1.0)) / WIDGET_GRID_COLS as f32;
    let slot_h = (inner_h - gap * (WIDGET_GRID_ROWS as f32 - 1.0)) / WIDGET_GRID_ROWS as f32;
    let grid_w = slot_w * WIDGET_GRID_COLS as f32 + gap * (WIDGET_GRID_COLS as f32 - 1.0);
    let grid_h = slot_h * WIDGET_GRID_ROWS as f32 + gap * (WIDGET_GRID_ROWS as f32 - 1.0);
    let grid_x = cap_x + (cap_w - grid_w) / 2.0;
    let grid_y = cap_y + (cap_h - grid_h) / 2.0;

    WidgetGridGeom {
        cap_x,
        cap_y,
        cap_w,
        cap_h,
        cap_scale,
        slot_w,
        slot_h,
        gap,
        grid_x,
        grid_y,
    }
}

pub fn widget_preview_hit_test(
    mx: f32,
    my: f32,
    item_y: f32,
    width: f32,
    expanded_width: f32,
    expanded_height: f32,
) -> WidgetPreviewHit {
    let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
    let py = item_y + (SettingsItem::WidgetPreview.height() - WIDGET_PREVIEW_H) / 2.0;
    let library_panel_y = py + WIDGET_ISLAND_PANEL_H + 12.0;

    let source_y = library_panel_y + 32.0;
    let source_w = 84.0;
    let source_h = 38.0;
    let source_gap = 12.0;
    let sources = [WidgetKind::Clock, WidgetKind::Status, WidgetKind::Weather];
    for (idx, kind) in sources.iter().enumerate() {
        let source_x = row_x + idx as f32 * (source_w + source_gap);
        if in_rect(mx, my, source_x, source_y, source_w, source_h) {
            return WidgetPreviewHit::Source(*kind);
        }
    }

    let geom = widget_grid_geom(item_y, width, expanded_width, expanded_height);
    for slot in 0..WIDGET_GRID_SLOTS {
        let (sx, sy, sw, sh) = geom.slot_rect(slot);
        if in_rect(mx, my, sx, sy, sw, sh) {
            return WidgetPreviewHit::Slot(slot);
        }
    }

    WidgetPreviewHit::None
}

pub fn hit_test(items: &[SettingsItem], mx: f32, my: f32, start_y: f32, width: f32) -> ClickResult {
    let mut y = start_y;
    let mut switch_idx = 0;
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
                    return ClickResult::Switch(switch_idx);
                }
                switch_idx += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM_Y: f32 = 80.0;
    const WIDTH: f32 = 486.0;
    const EXP_W: f32 = 360.0;
    const EXP_H: f32 = 200.0;

    #[test]
    fn widget_preview_hit_test_detects_all_nine_slots() {
        let geom = widget_grid_geom(ITEM_Y, WIDTH, EXP_W, EXP_H);
        for slot in 0..WIDGET_GRID_SLOTS {
            let (x, y, w, h) = geom.slot_rect(slot);
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            assert_eq!(
                widget_preview_hit_test(cx, cy, ITEM_Y, WIDTH, EXP_W, EXP_H),
                WidgetPreviewHit::Slot(slot),
                "center of slot {slot} should hit that slot"
            );
        }
    }

    #[test]
    fn widget_grid_slots_map_row_major() {
        let geom = widget_grid_geom(ITEM_Y, WIDTH, EXP_W, EXP_H);
        let (x0, y0, _, _) = geom.slot_rect(0);
        let (x1, y1, _, _) = geom.slot_rect(1);
        let (x3, y3, _, _) = geom.slot_rect(3);
        assert!(x1 > x0);
        assert!((y1 - y0).abs() < 0.01);
        assert!(y3 > y0);
        assert!((x3 - x0).abs() < 0.01);
    }

    #[test]
    fn widget_preview_hit_test_detects_sources() {
        let geom = widget_grid_geom(ITEM_Y, WIDTH, EXP_W, EXP_H);
        let py = ITEM_Y + (SettingsItem::WidgetPreview.height() - WIDGET_PREVIEW_H) / 2.0;
        let library_panel_y = py + WIDGET_ISLAND_PANEL_H + 12.0;
        let source_y = library_panel_y + 32.0 + 19.0;
        let row_x = CONTENT_PADDING + GROUP_INNER_PAD;
        assert!(source_y > geom.cap_y + geom.cap_h);
        assert_eq!(
            widget_preview_hit_test(row_x + 40.0, source_y, ITEM_Y, WIDTH, EXP_W, EXP_H),
            WidgetPreviewHit::Source(WidgetKind::Clock)
        );
        assert_eq!(
            widget_preview_hit_test(row_x + 136.0, source_y, ITEM_Y, WIDTH, EXP_W, EXP_H),
            WidgetPreviewHit::Source(WidgetKind::Status)
        );
        assert_eq!(
            widget_preview_hit_test(row_x + 232.0, source_y, ITEM_Y, WIDTH, EXP_W, EXP_H),
            WidgetPreviewHit::Source(WidgetKind::Weather)
        );
    }

    #[test]
    fn widget_preview_hit_test_ignores_points_outside_interactive_regions() {
        assert_eq!(
            widget_preview_hit_test(4.0, 90.0, ITEM_Y, WIDTH, EXP_W, EXP_H),
            WidgetPreviewHit::None
        );
    }
}
