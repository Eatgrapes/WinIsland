use skia_safe::Color;

pub const ROW_HEIGHT: f32 = 50.0;
pub const CARD_MARGIN: f32 = 20.0;
pub const CARD_HEIGHT: f32 = 42.0;
pub const CARD_RADIUS: f32 = 10.0;
pub const LABEL_X: f32 = 35.0;
pub const LABEL_Y_OFFSET: f32 = 21.0;
pub const SWITCH_X: f32 = 326.0;
pub const SWITCH_Y_OFFSET: f32 = 3.0;
pub const BTN_DEC_X: f32 = 270.0;
pub const BTN_INC_X: f32 = 345.0;
pub const BTN_Y_OFFSET: f32 = 2.0;
pub const VALUE_CENTER_X: f32 = 325.0;

pub enum SettingsItem {
    Stepper {
        label: String,
        value: String,
        enabled: bool,
    },
    Switch {
        label: String,
        #[allow(dead_code)]
        on: bool,
    },
    TextButton {
        label: String,
        btn_label: String,
        btn_x: f32,
        btn_w: f32,
    },
    FontPicker {
        label: String,
        btn_label: String,
        reset_label: Option<String>,
    },
    CenterLink {
        label: String,
        color: Color,
    },
    Title {
        text: String,
        size: f32,
    },
    CenterText {
        text: String,
        size: f32,
        color: Color,
    },
    SourceSelect {
        label: String,
        options: Vec<(String, f32, f32, bool)>,
        enabled: bool,
    },
    SectionHeader {
        label: String,
        btn: Option<(String, f32, f32, bool)>,
    },
    Label {
        label: String,
        enabled: bool,
    },
}

impl SettingsItem {
    pub fn height(&self) -> f32 {
        match self {
            SettingsItem::Title { .. } => ROW_HEIGHT,
            SettingsItem::CenterText { .. } => 35.0,
            SettingsItem::SectionHeader { .. } => 35.0,
            SettingsItem::CenterLink { .. } => 40.0,
            _ => ROW_HEIGHT,
        }
    }

    #[allow(dead_code)]
    pub fn has_card(&self) -> bool {
        matches!(self,
            SettingsItem::Stepper { .. } |
            SettingsItem::Switch { .. } |
            SettingsItem::TextButton { .. } |
            SettingsItem::FontPicker { .. } |
            SettingsItem::SourceSelect { .. } |
            SettingsItem::Label { .. }
        )
    }
}
