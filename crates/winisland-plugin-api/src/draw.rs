use std::ffi::c_void;

use crate::{ByteSliceV1, INTERFACE_VERSION_1, Utf8SliceV1};

/// Widget render callback invoked synchronously by the host on the render thread.
///
/// The context is valid only for the duration of this call. The plugin must not
/// retain the pointer or any value derived from it after returning.
pub type WidgetDrawFnV1 =
    unsafe extern "C" fn(callback_data: *mut c_void, ctx: *const WidgetDrawContextV1);

/// Rendering context handed to a plugin widget's `on_draw` callback.
///
/// Coordinates are logical and relative to the widget slot's top-left corner;
/// the host applies `scale` and `alpha` automatically inside every draw
/// function. `canvas_handle` is an opaque host-owned token — the plugin must
/// pass it back unchanged and must never dereference it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WidgetDrawContextV1 {
    /// Must be `size_of::<WidgetDrawContextV1>()`.
    pub struct_size: u32,
    /// Must be `INTERFACE_VERSION_1`.
    pub version: u32,
    /// Logical slot footprint width (span columns plus gaps).
    pub width: f32,
    /// Logical slot footprint height (span rows plus gaps).
    pub height: f32,
    /// Global scale factor, applied by the host.
    pub scale: f32,
    /// Island opacity (0-255), applied by the host.
    pub alpha: u8,
    /// Opaque host-owned canvas token. Never dereference.
    pub canvas_handle: *mut c_void,
    /// Drawing operations. Never null.
    pub draw: *const DrawApiV1,
}

impl WidgetDrawContextV1 {
    /// The host's drawing operations, or `None` if the pointer is null or the
    /// version does not match this ABI.
    ///
    /// # Safety
    /// `self.draw` must originate from the host and stay valid for this call.
    pub unsafe fn draw_api(&self) -> Option<&'static DrawApiV1> {
        // SAFETY: The host guarantees a valid operations pointer during on_draw.
        let draw = unsafe { self.draw.as_ref() }?;
        (draw.struct_size >= std::mem::size_of::<DrawApiV1>() as u32
            && draw.version == INTERFACE_VERSION_1)
            .then_some(draw)
    }
}

/// Drawing operations provided by the host.
///
/// Every function takes the context returned to the plugin's `on_draw`
/// callback as its first argument. All coordinates are logical slot
/// coordinates; all colors are `0xAARRGGBB`. The host applies the context's
/// `scale` and `alpha` to every operation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DrawApiV1 {
    pub struct_size: u32,
    pub version: u32,
    /// Draw a single line of text. `bold` is 0 or 1. `y` is the text top
    /// (ascent line), not the baseline. The font is managed by the host.
    pub draw_text: Option<
        unsafe extern "C" fn(
            ctx: *const WidgetDrawContextV1,
            x: f32,
            y: f32,
            text: Utf8SliceV1,
            size: f32,
            bold: u8,
            color: u32,
        ),
    >,
    /// Measure the width of a text line in logical pixels.
    pub measure_text: Option<
        unsafe extern "C" fn(
            ctx: *const WidgetDrawContextV1,
            text: Utf8SliceV1,
            size: f32,
            bold: u8,
        ) -> f32,
    >,
    /// Fill a rectangle.
    pub draw_rect: Option<
        unsafe extern "C" fn(
            ctx: *const WidgetDrawContextV1,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            color: u32,
        ),
    >,
    /// Fill a rounded rectangle.
    pub draw_round_rect: Option<
        unsafe extern "C" fn(
            ctx: *const WidgetDrawContextV1,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            radius: f32,
            color: u32,
        ),
    >,
    /// Fill a circle.
    pub draw_circle: Option<
        unsafe extern "C" fn(ctx: *const WidgetDrawContextV1, cx: f32, cy: f32, r: f32, color: u32),
    >,
    /// Stroke a line.
    pub draw_line: Option<
        unsafe extern "C" fn(
            ctx: *const WidgetDrawContextV1,
            x1: f32,
            y1: f32,
            x2: f32,
            y2: f32,
            stroke_width: f32,
            color: u32,
        ),
    >,
    /// Stroke an arc (progress ring). Angles are degrees, 0 at 3 o'clock,
    /// increasing clockwise.
    pub draw_arc: Option<
        unsafe extern "C" fn(
            ctx: *const WidgetDrawContextV1,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            start_angle: f32,
            sweep_angle: f32,
            stroke_width: f32,
            color: u32,
        ),
    >,
    /// Draw raw non-premultiplied RGBA8 pixels into the given logical rect.
    /// The host applies the context's alpha to the whole image.
    pub draw_image: Option<
        unsafe extern "C" fn(
            ctx: *const WidgetDrawContextV1,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            bitmap: ByteSliceV1,
            bitmap_width: u32,
            bitmap_height: u32,
        ),
    >,
    /// Push the current plugin transform. Must be balanced with `restore`.
    pub save: Option<unsafe extern "C" fn(ctx: *const WidgetDrawContextV1)>,
    /// Pop a plugin transform pushed with `save`.
    pub restore: Option<unsafe extern "C" fn(ctx: *const WidgetDrawContextV1)>,
    /// Translate subsequent drawing by logical pixels.
    pub translate: Option<unsafe extern "C" fn(ctx: *const WidgetDrawContextV1, dx: f32, dy: f32)>,
}
