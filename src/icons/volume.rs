use skia_safe::{Canvas, Color, Paint, Path, Point, paint};

const SPEAKER_PATH: &str = "M39.389,13.769 L22.235,28.606 L6,28.606 L6,47.699 L21.989,47.699 L39.389,62.75 L39.389,13.769z";
const WAVES_PATH: &str =
    "M48,27.6a19.5,19.5 0 0 1 0,21.4M55.1,20.5a30,30 0 0 1 0,35.6M61.6,14a38.8,38.8 0 0 1 0,48.6";

pub fn draw_volume_icon(
    canvas: &Canvas,
    center: Point,
    size: f32,
    alpha: u8,
    muted: bool,
    color: Color,
) {
    let mut fill = Paint::default();
    fill.set_anti_alias(true);
    fill.set_color(Color::from_argb(alpha, color.r(), color.g(), color.b()));

    canvas.save();
    canvas.translate((center.x, center.y));
    let scale = size / 68.0;
    canvas.scale((scale, scale));
    canvas.translate((-34.0, -38.0));

    if let Some(path) = Path::from_svg(SPEAKER_PATH) {
        canvas.draw_path(&path, &fill);
    }

    if muted {
        let mut slash = fill;
        slash.set_style(paint::Style::Stroke);
        slash.set_stroke_width(4.0);
        slash.set_stroke_cap(paint::Cap::Round);
        canvas.draw_line((46.0, 29.0), (62.0, 48.0), &slash);
        canvas.draw_line((62.0, 29.0), (46.0, 48.0), &slash);
    } else if let Some(path) = Path::from_svg(WAVES_PATH) {
        let mut waves = fill;
        waves.set_style(paint::Style::Stroke);
        waves.set_stroke_width(3.4);
        waves.set_stroke_cap(paint::Cap::Round);
        canvas.draw_path(&path, &waves);
    }

    canvas.restore();
}
