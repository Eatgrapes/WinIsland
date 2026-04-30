use super::items::*;

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
    AppItem(usize),
}

fn in_rect(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

pub fn hit_test(items: &[SettingsItem], mx: f32, my: f32, start_y: f32, width: f32) -> ClickResult {
    let mut y = start_y;
    let mut idx = 0;
    let mut switch_idx = 0;
    let content_w = width - CONTENT_PADDING * 2.0;

    for item in items {
        match item {
            SettingsItem::RowStepper { enabled, .. } => {
                if *enabled {
                    let cy = y + ROW_HEIGHT / 2.0;
                    let btn_inc_x =
                        CONTENT_PADDING + content_w - GROUP_INNER_PAD - STEPPER_BTN_SIZE;
                    let btn_dec_x = btn_inc_x - STEPPER_BTN_SIZE - 60.0;
                    let btn_y = cy - STEPPER_BTN_SIZE / 2.0;
                    if in_rect(mx, my, btn_dec_x, btn_y, STEPPER_BTN_SIZE, STEPPER_BTN_SIZE) {
                        return ClickResult::StepperDec(idx);
                    }
                    if in_rect(mx, my, btn_inc_x, btn_y, STEPPER_BTN_SIZE, STEPPER_BTN_SIZE) {
                        return ClickResult::StepperInc(idx);
                    }
                }
            }
            SettingsItem::RowSwitch { enabled, .. } => {
                if *enabled {
                    let cy = y + ROW_HEIGHT / 2.0;
                    let toggle_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - TOGGLE_W;
                    let toggle_y = cy - TOGGLE_H / 2.0;
                    if in_rect(mx, my, toggle_x, toggle_y, TOGGLE_W, TOGGLE_H) {
                        return ClickResult::Switch(switch_idx);
                    }
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
            SettingsItem::RowSourceSelect { enabled, .. } => {
                if *enabled {
                    let cy = y + ROW_HEIGHT / 2.0;
                    let btn_x = CONTENT_PADDING + content_w - GROUP_INNER_PAD - POPUP_BTN_W;
                    let btn_y = cy - POPUP_BTN_H / 2.0;
                    if in_rect(mx, my, btn_x, btn_y, POPUP_BTN_W, POPUP_BTN_H) {
                        return ClickResult::SourceButton(idx);
                    }
                }
            }
            SettingsItem::RowAppItem { enabled, .. } => {
                if *enabled && in_rect(mx, my, CONTENT_PADDING, y, content_w, ROW_HEIGHT) {
                    return ClickResult::AppItem(idx);
                }
            }
            SettingsItem::CenterLink { .. } => {
                if mx >= width / 2.0 - 100.0
                    && mx <= width / 2.0 + 100.0
                    && my >= y
                    && my <= y + 40.0
                {
                    return ClickResult::CenterLink(idx);
                }
            }
            _ => {}
        }
        y += item.height();
        idx += 1;
    }
    ClickResult::None
}

pub fn hover_test(items: &[SettingsItem], mx: f32, my: f32, start_y: f32, width: f32) -> bool {
    hit_test(items, mx, my, start_y, width) != ClickResult::None
}
