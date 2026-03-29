use super::items::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ClickResult {
    None,
    Switch(usize),
    StepperDec(usize),
    StepperInc(usize),
    TextButton(usize),
    FontSelect(usize),
    FontReset(usize),
    CenterLink(usize),
    SourceOption(usize, usize),
    SectionButton(usize),
    Label(usize),
}

fn in_rect(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
    mx >= x && mx <= x + w && my >= y && my <= y + h
}

pub fn hit_test(items: &[SettingsItem], mx: f32, my: f32, start_y: f32, _width: f32) -> ClickResult {
    let mut y = start_y;
    let mut idx = 0;

    for item in items {
        match item {
            SettingsItem::Stepper { enabled, .. } => {
                if *enabled {
                    if in_rect(mx, my, BTN_DEC_X, y + BTN_Y_OFFSET, 28.0, 28.0) {
                        return ClickResult::StepperDec(idx);
                    }
                    if in_rect(mx, my, BTN_INC_X, y + BTN_Y_OFFSET, 28.0, 28.0) {
                        return ClickResult::StepperInc(idx);
                    }
                }
            }
            SettingsItem::Switch { .. } => {
                if in_rect(mx, my, SWITCH_X, y + SWITCH_Y_OFFSET, 48.0, 26.0) {
                    return ClickResult::Switch(idx);
                }
            }
            SettingsItem::TextButton { btn_x, btn_w, .. } => {
                if in_rect(mx, my, *btn_x, y + 3.0, *btn_w, 26.0) {
                    return ClickResult::TextButton(idx);
                }
            }
            SettingsItem::FontPicker { reset_label, .. } => {
                if in_rect(mx, my, 310.0, y + 3.0, 65.0, 26.0) {
                    return ClickResult::FontSelect(idx);
                }
                if reset_label.is_some() && in_rect(mx, my, 235.0, y + 3.0, 65.0, 26.0) {
                    return ClickResult::FontReset(idx);
                }
            }
            SettingsItem::CenterLink { .. } => {
                if mx >= _width / 2.0 - 100.0 && mx <= _width / 2.0 + 100.0
                    && my >= y && my <= y + 40.0
                {
                    return ClickResult::CenterLink(idx);
                }
            }
            SettingsItem::SourceSelect { options, enabled, .. } => {
                if *enabled {
                    for (opt_idx, (_, ox, ow, _)) in options.iter().enumerate() {
                        if in_rect(mx, my, *ox, y + 3.0, *ow, 22.0) {
                            return ClickResult::SourceOption(idx, opt_idx);
                        }
                    }
                }
            }
            SettingsItem::SectionHeader { btn, .. } => {
                if let Some((_, bx, bw, enabled)) = btn {
                    if *enabled && in_rect(mx, my, *bx, y, *bw, 24.0) {
                        return ClickResult::SectionButton(idx);
                    }
                }
            }
            SettingsItem::Label { .. } => {
                if in_rect(mx, my, CARD_MARGIN, y - 5.0, _width - CARD_MARGIN * 2.0, CARD_HEIGHT) {
                    return ClickResult::Label(idx);
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
