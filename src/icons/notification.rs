use skia_safe::{Canvas, Color, Paint, Path};

pub fn draw_notification_icon(canvas: &Canvas, cx: f32, cy: f32, alpha: u8, scale: f32) {
    let mut paint = Paint::default();
    paint.set_color(Color::from_argb(alpha, 255, 255, 255));
    paint.set_anti_alias(true);
    paint.set_style(skia_safe::paint::Style::Fill);

    let path_data = "M512 896c-53.6 0-97.2-43.6-97.2-97.2h194.4c0 53.6-43.6 97.2-97.2 97.2zM832 736H192v-32l64-64V448c0-97.6 65.2-180 156-206.4V224c0-55.2 44.8-96 100-96s100 40.8 100 96v17.6c90.8 26.4 156 108.8 156 206.4v192l64 64v32z";
    
    if let Some(path) = Path::from_svg(path_data) {
        canvas.save();
        canvas.translate((cx, cy));
        let s = 0.024 * scale;
        canvas.scale((s, s));
        canvas.translate((-512.0, -512.0));
        canvas.draw_path(&path, &paint);
        canvas.restore();
    }
}
