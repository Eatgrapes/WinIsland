use skia_safe::{
    ImageInfo, Surface,
    gpu::{self, Budgeted, DirectContext, SurfaceOrigin},
};

pub(crate) fn render_surface(context: &mut DirectContext, info: &ImageInfo) -> Option<Surface> {
    gpu::surfaces::render_target(
        context,
        Budgeted::Yes,
        info,
        None,
        SurfaceOrigin::TopLeft,
        None,
        Some(false),
        Some(false),
    )
}
