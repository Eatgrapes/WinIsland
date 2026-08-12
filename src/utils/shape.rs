use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::OnceLock;

use skia_safe::{Path, PathBuilder, Rect};

const G3_BLEND_EXTENT: f32 = 0.3;
const G3_BLEND_SEGMENTS: usize = 8;
const CORNER_COUNT: usize = 4;
const PATH_CACHE_CAPACITY: usize = 8;

struct G3Geometry {
    blend_coefficients: [f32; 4],
    blend_points: [(f32, f32); G3_BLEND_SEGMENTS],
    arc_control: (f32, f32),
    arc_end: (f32, f32),
    arc_weight: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PathKey {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    radius: u32,
}

struct CachedPath {
    key: PathKey,
    path: Path,
}

thread_local! {
    static PATH_CACHE: RefCell<VecDeque<CachedPath>> = const { RefCell::new(VecDeque::new()) };
}

fn evaluate_blend(x: f32, coefficients: [f32; 4]) -> f32 {
    let t = (x / G3_BLEND_EXTENT).clamp(0.0, 1.0);
    let t2 = t * t;
    let [a4, a5, a6, a7] = coefficients;
    t2 * t2 * (a4 + t * (a5 + t * (a6 + t * a7)))
}

fn solve_g3_blend_coefficients(
    position: f32,
    slope: f32,
    curvature: f32,
    curvature_rate: f32,
) -> [f32; 4] {
    [
        35.0 * position - 15.0 * slope + 2.5 * curvature - curvature_rate / 6.0,
        -84.0 * position + 39.0 * slope - 7.0 * curvature + curvature_rate / 2.0,
        70.0 * position - 34.0 * slope + 6.5 * curvature - curvature_rate / 2.0,
        -20.0 * position + 10.0 * slope - 2.0 * curvature + curvature_rate / 6.0,
    ]
}

fn g3_geometry() -> &'static G3Geometry {
    static GEOMETRY: OnceLock<G3Geometry> = OnceLock::new();
    GEOMETRY.get_or_init(|| {
        let extent = G3_BLEND_EXTENT;
        let circle_root = (1.0 - extent * extent).sqrt();
        let position = 1.0 - circle_root;
        let slope = extent * extent / circle_root;
        let curvature = extent * extent / circle_root.powi(3);
        let curvature_rate = 3.0 * extent.powi(4) / circle_root.powi(5);
        let blend_coefficients =
            solve_g3_blend_coefficients(position, slope, curvature, curvature_rate);
        let blend_points = std::array::from_fn(|index| {
            let x = extent * (index + 1) as f32 / G3_BLEND_SEGMENTS as f32;
            (x, evaluate_blend(x, blend_coefficients))
        });
        let arc_sweep = std::f32::consts::FRAC_PI_2 - 2.0 * extent.asin();
        let arc_weight = (arc_sweep / 2.0).cos();
        let control_x = std::f32::consts::FRAC_1_SQRT_2 / arc_weight;

        G3Geometry {
            blend_coefficients,
            blend_points,
            arc_control: (control_x, 1.0 - control_x),
            arc_end: (circle_root, 1.0 - extent),
            arc_weight,
        }
    })
}

fn g3_blend_y(x: f32) -> f32 {
    evaluate_blend(x, g3_geometry().blend_coefficients)
}

fn append_g3_corner(builder: &mut PathBuilder, point_at: impl Fn(f32, f32) -> (f32, f32)) {
    let geometry = g3_geometry();
    for (x, y) in geometry.blend_points {
        builder.line_to(point_at(x, y));
    }

    builder.conic_to(
        point_at(geometry.arc_control.0, geometry.arc_control.1),
        point_at(geometry.arc_end.0, geometry.arc_end.1),
        geometry.arc_weight,
    );

    for (x, y) in geometry.blend_points.into_iter().rev() {
        if x < G3_BLEND_EXTENT {
            builder.line_to(point_at(1.0 - y, 1.0 - x));
        }
    }
    builder.line_to(point_at(1.0, 1.0));
}

pub(crate) fn g3_corner_contains(x: f64, y: f64) -> bool {
    let extent = G3_BLEND_EXTENT as f64;
    if x <= extent {
        1.0 - y >= g3_blend_y(x as f32) as f64
    } else if y <= extent {
        1.0 - x >= g3_blend_y(y as f32) as f64
    } else {
        x * x + y * y <= 1.0
    }
}

pub(crate) fn g3_rounded_rect_path(rect: Rect, radius: f32) -> Path {
    if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return Path::default();
    }

    let radius = radius
        .max(0.0)
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0);
    let key = PathKey {
        left: rect.left().to_bits(),
        top: rect.top().to_bits(),
        right: rect.right().to_bits(),
        bottom: rect.bottom().to_bits(),
        radius: radius.to_bits(),
    };
    if let Some(path) = PATH_CACHE.with(|cache| {
        cache
            .borrow()
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.path.clone())
    }) {
        return path;
    }

    let mut builder = PathBuilder::new();
    if radius == 0.0 {
        builder.add_rect(rect, None, None);
    } else {
        let left = rect.left();
        let top = rect.top();
        let right = rect.right();
        let bottom = rect.bottom();

        let points_per_corner = G3_BLEND_SEGMENTS * 2 + 2;
        let verbs_per_corner = G3_BLEND_SEGMENTS * 2 + 1;
        builder.inc_reserve(
            1 + CORNER_COUNT + CORNER_COUNT * points_per_corner,
            2 + CORNER_COUNT + CORNER_COUNT * verbs_per_corner,
            CORNER_COUNT,
        );
        builder.move_to((left + radius, top));
        builder.line_to((right - radius, top));
        append_g3_corner(&mut builder, |x, y| {
            (right - radius + radius * x, top + radius * y)
        });
        builder.line_to((right, bottom - radius));
        append_g3_corner(&mut builder, |x, y| {
            (right - radius * y, bottom - radius + radius * x)
        });
        builder.line_to((left + radius, bottom));
        append_g3_corner(&mut builder, |x, y| {
            (left + radius - radius * x, bottom - radius * y)
        });
        builder.line_to((left, top + radius));
        append_g3_corner(&mut builder, |x, y| {
            (left + radius * y, top + radius - radius * x)
        });
        builder.close();
    }

    let path = builder.detach();
    PATH_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.push_front(CachedPath {
            key,
            path: path.clone(),
        });
        cache.truncate(PATH_CACHE_CAPACITY);
    });
    path
}
