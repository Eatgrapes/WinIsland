use std::cell::RefCell;
use std::time::{Duration, Instant};

use skia_safe::{Canvas, Color, Paint, Rect};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

use super::{draw_widget_rounded_background, draw_widget_text_centered};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const CPU_COLOR: Color = Color::from_rgb(50, 190, 246);
const RAM_COLOR: Color = Color::from_rgb(175, 82, 222);
const WARNING_COLOR: Color = Color::from_rgb(255, 159, 10);
const CRITICAL_COLOR: Color = Color::from_rgb(255, 69, 58);

#[derive(Clone, Copy, Default)]
struct CpuTimes {
    idle: u64,
    total: u64,
}

#[derive(Default)]
struct ResourceUsageCache {
    sampled_at: Option<Instant>,
    previous_cpu: Option<CpuTimes>,
    cpu: Option<f32>,
    ram: Option<f32>,
    cpu_text: String,
    ram_text: String,
}

impl ResourceUsageCache {
    fn refresh_if_due(&mut self) {
        if self
            .sampled_at
            .is_some_and(|sampled_at| sampled_at.elapsed() < SAMPLE_INTERVAL)
        {
            return;
        }
        self.sampled_at = Some(Instant::now());

        if let Some(current) = read_cpu_times() {
            if let Some(previous) = self.previous_cpu {
                let total = current.total.saturating_sub(previous.total);
                let idle = current.idle.saturating_sub(previous.idle);
                if total > 0 {
                    self.cpu = Some((1.0 - idle as f32 / total as f32).clamp(0.0, 1.0));
                }
            }
            self.previous_cpu = Some(current);
        }
        if let Some(ram) = read_ram_usage() {
            self.ram = Some(ram);
        }
        update_percent_text(&mut self.cpu_text, self.cpu);
        update_percent_text(&mut self.ram_text, self.ram);
    }
}

thread_local! {
    static RESOURCE_USAGE: RefCell<ResourceUsageCache> = RefCell::new(ResourceUsageCache::default());
}

fn filetime_ticks(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

fn read_cpu_times() -> Option<CpuTimes> {
    let mut idle = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: All pointers reference initialized FILETIME values that remain valid for the
    // duration of this synchronous call.
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    Some(CpuTimes {
        idle: filetime_ticks(idle),
        total: filetime_ticks(kernel).saturating_add(filetime_ticks(user)),
    })
}

fn read_ram_usage() -> Option<f32> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: status has the required dwLength and remains valid for this synchronous call.
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    Some((status.dwMemoryLoad as f32 / 100.0).clamp(0.0, 1.0))
}

fn update_percent_text(text: &mut String, value: Option<f32>) {
    if let Some(value) = value {
        *text = format!("{:.0}%", value * 100.0);
    } else if text.is_empty() {
        text.push('—');
    }
}

fn blend_color(from: Color, to: Color, amount: f32) -> Color {
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount) as u8;
    Color::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
}

fn usage_color(base: Color, usage: f32) -> Color {
    if usage <= 0.75 {
        base
    } else if usage <= 0.9 {
        blend_color(base, WARNING_COLOR, (usage - 0.75) / 0.15)
    } else {
        blend_color(WARNING_COLOR, CRITICAL_COLOR, (usage - 0.9) / 0.1)
    }
}

fn alpha_color(color: Color, alpha: u8) -> Color {
    Color::from_argb(alpha, color.r(), color.g(), color.b())
}

#[allow(clippy::too_many_arguments)]
fn draw_metric(
    canvas: &Canvas,
    bounds: Rect,
    label: &str,
    value: Option<f32>,
    value_text: &str,
    base_color: Color,
    scale: f32,
    alpha: u8,
    text_color: Color,
) {
    let center_x = bounds.center_x();
    let center_y = bounds.top() + bounds.height() * 0.43;
    let diameter = (bounds.height() * 0.58)
        .min(bounds.width() * 0.62)
        .max(20.0 * scale);
    let ring = Rect::from_xywh(
        center_x - diameter / 2.0,
        center_y - diameter / 2.0,
        diameter,
        diameter,
    );
    let usage = value.unwrap_or(0.0);
    let accent = usage_color(base_color, usage);

    let mut glow = Paint::default();
    glow.set_anti_alias(true);
    glow.set_color(alpha_color(accent, (alpha as f32 * 0.07) as u8));
    canvas.draw_circle((center_x, center_y), diameter * 0.57, &glow);

    let mut ring_paint = Paint::default();
    ring_paint.set_anti_alias(true);
    ring_paint.set_style(skia_safe::paint::Style::Stroke);
    ring_paint.set_stroke_width((3.0 * scale).min(diameter * 0.12));
    ring_paint.set_stroke_cap(skia_safe::paint::Cap::Round);
    ring_paint.set_color(alpha_color(text_color, (alpha as f32 * 0.13) as u8));
    canvas.draw_circle((center_x, center_y), diameter / 2.0, &ring_paint);

    if value.is_some() && usage > 0.0 {
        ring_paint.set_color(alpha_color(accent, (alpha as f32 * 0.92) as u8));
        canvas.draw_arc(ring, -90.0, usage * 360.0, false, &ring_paint);
    }

    let mut value_paint = Paint::default();
    value_paint.set_anti_alias(true);
    value_paint.set_color(alpha_color(text_color, alpha));
    draw_widget_text_centered(
        canvas,
        value_text,
        Rect::from_xywh(
            center_x - diameter * 0.42,
            center_y - diameter * 0.24,
            diameter * 0.84,
            diameter * 0.48,
        ),
        (diameter * 0.27).clamp(7.0 * scale, 10.5 * scale),
        true,
        &value_paint,
    );

    let mut label_paint = Paint::default();
    label_paint.set_anti_alias(true);
    label_paint.set_color(alpha_color(text_color, (alpha as f32 * 0.58) as u8));
    draw_widget_text_centered(
        canvas,
        label,
        Rect::from_xywh(
            bounds.left(),
            bounds.top() + bounds.height() * 0.76,
            bounds.width(),
            bounds.height() * 0.16,
        ),
        (bounds.height() * 0.12).clamp(6.0 * scale, 8.0 * scale),
        true,
        &label_paint,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_resource_usage(
    canvas: &Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
    alpha: u8,
    text_color: Color,
    cpu: Option<f32>,
    ram: Option<f32>,
    cpu_text: &str,
    ram_text: &str,
) {
    draw_widget_rounded_background(canvas, x, y, w, h, scale, alpha);

    let mut divider = Paint::default();
    divider.set_anti_alias(true);
    divider.set_color(alpha_color(text_color, (alpha as f32 * 0.09) as u8));
    divider.set_stroke_width(scale.max(0.75));
    canvas.draw_line(
        (x + w / 2.0, y + h * 0.2),
        (x + w / 2.0, y + h * 0.8),
        &divider,
    );

    let inset = 3.0 * scale;
    let metric_w = (w - inset * 2.0) / 2.0;
    draw_metric(
        canvas,
        Rect::from_xywh(x + inset, y, metric_w, h),
        "CPU",
        cpu,
        cpu_text,
        CPU_COLOR,
        scale,
        alpha,
        text_color,
    );
    draw_metric(
        canvas,
        Rect::from_xywh(x + inset + metric_w, y, metric_w, h),
        "RAM",
        ram,
        ram_text,
        RAM_COLOR,
        scale,
        alpha,
        text_color,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_resource_usage_widget(
    canvas: &Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
    alpha: u8,
    text_color: Color,
) {
    RESOURCE_USAGE.with(|cell| {
        let mut cache = cell.borrow_mut();
        cache.refresh_if_due();
        draw_resource_usage(
            canvas,
            x,
            y,
            w,
            h,
            scale,
            alpha,
            text_color,
            cache.cpu,
            cache.ram,
            &cache.cpu_text,
            &cache.ram_text,
        );
    });
}

#[allow(clippy::too_many_arguments)]
pub fn draw_resource_usage_preview(
    canvas: &Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
    alpha: u8,
    text_color: Color,
) {
    draw_resource_usage(
        canvas,
        x,
        y,
        w,
        h,
        scale,
        alpha,
        text_color,
        Some(0.37),
        Some(0.62),
        "37%",
        "62%",
    );
}
