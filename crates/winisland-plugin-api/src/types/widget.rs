use crate::WidgetDrawFnV1;

/// Show this widget's compact representation in the mini island.
///
/// Reserved for future use; the host currently renders widgets on the
/// expanded widget page only.
pub const WIDGET_FLAG_SHOW_COMPACT: u32 = 1 << 0;

/// Widget content owned by a plugin resource.
///
/// The host places the widget on the expanded widget page grid and invokes
/// `on_draw` on every frame to render it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WidgetDataV1 {
    /// Must be `size_of::<WidgetDataV1>()`.
    pub struct_size: u32,
    /// Grid columns occupied (1-6).
    pub span_cols: u32,
    /// Grid rows occupied (1-3).
    pub span_rows: u32,
    /// Combination of `WIDGET_FLAG_*` values.
    pub flags: u32,
    /// Widget title. Max 255 bytes plus NUL.
    pub title: [u8; 256],
    /// Widget body text. Max 511 bytes plus NUL.
    pub body: [u8; 512],
    /// Render callback invoked synchronously on the render thread.
    pub on_draw: Option<WidgetDrawFnV1>,
    /// Opaque pointer passed back to `on_draw`.
    pub callback_data: *mut std::ffi::c_void,
}

impl Default for WidgetDataV1 {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            span_cols: 2,
            span_rows: 1,
            flags: 0,
            title: [0; 256],
            body: [0; 512],
            on_draw: None,
            callback_data: std::ptr::null_mut(),
        }
    }
}
