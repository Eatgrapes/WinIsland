use skia_safe::Color;

pub const CONTENT_PADDING: f32 = 20.0;
pub const ROW_HEIGHT: f32 = 44.0;
pub const GROUP_RADIUS: f32 = 10.0;
pub const GROUP_INNER_PAD: f32 = 16.0;
pub const SIDEBAR_PAD: f32 = 12.0;
pub const SIDEBAR_SEL_RADIUS: f32 = 6.0;

pub const TOGGLE_W: f32 = 38.0;
pub const TOGGLE_H: f32 = 22.0;
pub const TOGGLE_R: f32 = 11.0;
pub const TOGGLE_KNOB: f32 = 18.0;
pub const TOGGLE_INSET: f32 = 2.0;

pub const STEPPER_BTN_SIZE: f32 = 24.0;

pub const POPUP_BTN_W: f32 = 80.0;
pub const POPUP_BTN_H: f32 = 26.0;
pub const POPUP_BTN_R: f32 = 6.0;
pub const POPUP_ITEM_H: f32 = 28.0;
pub const POPUP_MENU_R: f32 = 8.0;
pub const POPUP_MENU_PAD: f32 = 4.0;

pub enum SettingsItem {
    PageTitle {
        text: String,
    },
    SectionHeader {
        label: String,
    },
    GroupStart,
    GroupEnd,
    RowStepper {
        label: String,
        value: String,
        enabled: bool,
    },
    RowSwitch {
        label: String,
        #[allow(dead_code)]
        on: bool,
        enabled: bool,
    },
    RowFontPicker {
        label: String,
        btn_label: String,
        reset_label: Option<String>,
    },
    RowSourceSelect {
        label: String,
        options: Vec<(String, bool)>,
        enabled: bool,
    },
    RowAppItem {
        label: String,
        active: bool,
        enabled: bool,
    },
    RowLabel {
        label: String,
    },
    RowPluginItem {
        name: String,
        version: String,
        author: String,
        description: String,
        enabled: bool,
        has_settings: bool,
    },
    RowPluginAction {
        label: String,
        color: Color,
    },
    CenterLink {
        label: String,
        color: Color,
    },
    CenterText {
        text: String,
        size: f32,
        color: Color,
    },
    Spacer {
        height: f32,
    },
}

impl SettingsItem {
    pub fn height(&self) -> f32 {
        match self {
            SettingsItem::PageTitle { .. } => 50.0,
            SettingsItem::SectionHeader { .. } => 30.0,
            SettingsItem::GroupStart | SettingsItem::GroupEnd => 0.0,
            SettingsItem::CenterLink { .. } => 40.0,
            SettingsItem::CenterText { .. } => 35.0,
            SettingsItem::Spacer { height } => *height,
            SettingsItem::RowPluginItem { .. } => 80.0,
            _ => ROW_HEIGHT,
        }
    }

    pub fn is_row(&self) -> bool {
        matches!(self,
            SettingsItem::RowStepper { .. } |
            SettingsItem::RowSwitch { .. } |
            SettingsItem::RowFontPicker { .. } |
            SettingsItem::RowSourceSelect { .. } |
            SettingsItem::RowAppItem { .. } |
            SettingsItem::RowLabel { .. } |
            SettingsItem::RowPluginItem { .. } |
            SettingsItem::RowPluginAction { .. }
        )
    }
}
