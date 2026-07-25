use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    AlphaType, Canvas, ClipOp, Color, ColorType, Data, FilterMode, FontStyle, ISize, Image,
    ImageInfo, MipmapMode, Paint, Rect, SamplingOptions,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::core::codex::{CodexPet, CodexSnapshot, CodexState, read_local_codex_logo_bytes};
use crate::utils::font::{DrawTextCachedParams, FontManager};

const CELL_WIDTH: f32 = 192.0;
const CELL_HEIGHT: f32 = 208.0;
const ATLAS_WIDTH: i32 = 1536;
const ATLAS_HEIGHT: i32 = 2288;
const COMPACT_HEIGHT_WITH_PET: f32 = 42.0;
const COMPACT_PET_WIDTH: f32 = 32.0;
const COMPACT_FONT_SIZE: f32 = 14.0;
const COMPACT_LOGO_SIZE: f32 = 18.0;
const COMPACT_GROUP_PADDING: f32 = 10.0;
const COMPACT_ITEM_GAP: f32 = 12.0;
const HEADER_HEIGHT_WITH_PET: f32 = 100.0;
const HEADER_HEIGHT_WITHOUT_PET: f32 = 72.0;
const HEADER_PET_WIDTH: f32 = 70.0;
const HEADER_PET_TEXT_GAP: f32 = 14.0;
const HEADER_TITLE_Y_WITH_PET: f32 = 43.0;
const HEADER_STATUS_Y_WITH_PET: f32 = 70.0;
const HEADER_TITLE_Y_WITHOUT_PET: f32 = 33.0;
const HEADER_STATUS_Y_WITHOUT_PET: f32 = 57.0;
const CONTENT_HORIZONTAL_PADDING: f32 = 24.0;
const CONTENT_TOP_PADDING: f32 = 16.0;
const CONTENT_BOTTOM_PADDING: f32 = 24.0;
const HEADER_TITLE_FONT_SIZE: f32 = 18.0;
const HEADER_STATUS_FONT_SIZE: f32 = 13.0;
const CONTENT_FONT_SIZE: f32 = 15.0;
const CONTENT_LINE_HEIGHT: f32 = 25.0;

const IDLE_FRAMES: &[u32] = &[1680, 660, 660, 840, 840, 1920];
const WAVING_FRAMES: &[u32] = &[140, 140, 140, 280];
const JUMPING_FRAMES: &[u32] = &[140, 140, 140, 140, 280];
const FAILED_FRAMES: &[u32] = &[140, 140, 140, 140, 140, 140, 140, 240];
const WAITING_FRAMES: &[u32] = &[150, 150, 150, 150, 150, 260];
const RUNNING_FRAMES: &[u32] = &[120, 120, 120, 120, 120, 220];
const REVIEW_FRAMES: &[u32] = &[150, 150, 150, 150, 150, 280];
const PET_IMAGE_RETRY_INTERVAL: Duration = Duration::from_secs(3);
const LOGO_IMAGE_RETRY_INTERVAL: Duration = Duration::from_secs(3);

thread_local! {
    static PET_IMAGE: RefCell<Option<CachedPetImage>> = const { RefCell::new(None) };
    static LOGO_IMAGE: RefCell<Option<CachedLogoImage>> = const { RefCell::new(None) };
    static TEXT_LAYOUT: RefCell<Option<TextLayout>> = const { RefCell::new(None) };
}

struct CachedPetImage {
    pet: CodexPet,
    image: Option<Image>,
    last_attempt: Instant,
}

struct CachedLogoImage {
    image: Option<Image>,
    last_attempt: Instant,
}

struct PetDrawParams<'a> {
    canvas: &'a Canvas,
    snapshot: &'a CodexSnapshot,
    pet: &'a CodexPet,
    x: f32,
    y: f32,
    size: f32,
    animation_time_secs: f32,
    alpha: f32,
}

struct TextLayout {
    key: u64,
    width: f32,
    font_size: f32,
    lines: Vec<String>,
}

#[derive(Clone, Copy)]
pub struct CodexExpandedMetrics {
    pub desired_height: f32,
    pub scroll_max: f32,
}

pub struct DrawCodexCompactParams<'a> {
    pub canvas: &'a Canvas,
    pub snapshot: &'a CodexSnapshot,
    pub pet: Option<&'a CodexPet>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub font_size: f32,
    pub animation_time_secs: f32,
    pub alpha: f32,
}

pub struct DrawCodexExpandedParams<'a> {
    pub canvas: &'a Canvas,
    pub snapshot: &'a CodexSnapshot,
    pub pet: Option<&'a CodexPet>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub font_size: f32,
    pub animation_time_secs: f32,
    pub scroll_offset: f32,
    pub alpha: f32,
}

#[derive(Clone, Copy)]
enum PetAnimation {
    Idle,
    Waving,
    Jumping,
    Failed,
    Waiting,
    Running,
    Review,
}

impl PetAnimation {
    fn row_and_frames(self) -> (u32, &'static [u32]) {
        match self {
            Self::Idle => (0, IDLE_FRAMES),
            Self::Waving => (3, WAVING_FRAMES),
            Self::Jumping => (4, JUMPING_FRAMES),
            Self::Failed => (5, FAILED_FRAMES),
            Self::Waiting => (6, WAITING_FRAMES),
            Self::Running => (7, RUNNING_FRAMES),
            Self::Review => (8, REVIEW_FRAMES),
        }
    }
}

pub fn compact_width(snapshot: &CodexSnapshot, scale: f32, has_pet: bool, font_size: f32) -> f32 {
    let font_size = scaled_font_size(COMPACT_FONT_SIZE, font_size, scale);
    let label_width = FontManager::global().measure_text_cached(
        status_label(snapshot),
        font_size,
        FontStyle::bold(),
    );
    if has_pet {
        (COMPACT_GROUP_PADDING * 2.0 + COMPACT_PET_WIDTH + COMPACT_LOGO_SIZE) * scale
            + label_width
            + COMPACT_ITEM_GAP * 2.0 * scale
    } else {
        label_width + 48.0 * scale
    }
}

pub const fn compact_height(scale: f32, has_pet: bool) -> f32 {
    if has_pet {
        COMPACT_HEIGHT_WITH_PET * scale
    } else {
        0.0
    }
}

pub fn compact_size(
    snapshot: &CodexSnapshot,
    base_width: f32,
    base_height: f32,
    scale: f32,
    has_pet: bool,
    font_size: f32,
) -> (f32, f32) {
    (
        (base_width * scale).max(compact_width(snapshot, scale, has_pet, font_size)),
        (base_height * scale).max(compact_height(scale, has_pet)),
    )
}

pub const fn expanded_header_height(scale: f32, has_pet: bool) -> f32 {
    if has_pet {
        HEADER_HEIGHT_WITH_PET * scale
    } else {
        HEADER_HEIGHT_WITHOUT_PET * scale
    }
}

pub fn expanded_metrics(
    snapshot: &CodexSnapshot,
    has_pet: bool,
    width: f32,
    max_height: f32,
    scale: f32,
    font_size: f32,
) -> CodexExpandedMetrics {
    let content_width = (width - CONTENT_HORIZONTAL_PADDING * 2.0 * scale).max(1.0);
    let content_font_size = scaled_font_size(CONTENT_FONT_SIZE, font_size, scale);
    let line_height = content_line_height(font_size, scale);
    let header_height = expanded_header_height(scale, has_pet);
    let line_count = with_text_layout(snapshot, content_width, content_font_size, |lines| {
        lines.len()
    });
    let content_height = (line_count.max(1) as f32) * line_height;
    let content_padding = (CONTENT_TOP_PADDING + CONTENT_BOTTOM_PADDING) * scale;
    let minimum_height = header_height + content_padding + line_height;
    let natural_height = header_height + content_height + content_padding;
    let desired_height = natural_height.max(minimum_height).min(max_height.max(1.0));
    let visible_height = (desired_height - header_height - content_padding).max(0.0);
    CodexExpandedMetrics {
        desired_height,
        scroll_max: (content_height - visible_height).max(0.0),
    }
}

pub fn draw_compact(params: DrawCodexCompactParams<'_>) -> bool {
    if params.alpha <= 0.01 {
        return false;
    }

    let font_size = scaled_font_size(COMPACT_FONT_SIZE, params.font_size, params.scale);
    let label = status_label(params.snapshot);
    let label_width =
        FontManager::global().measure_text_cached(label, font_size, FontStyle::bold());
    let pet_width = COMPACT_PET_WIDTH * params.scale;
    let pet_height = pet_display_height(pet_width);
    let logo_size = COMPACT_LOGO_SIZE * params.scale;
    let item_gap = COMPACT_ITEM_GAP * params.scale;
    let group_width = pet_width + item_gap + label_width + item_gap + logo_size;
    let pet_x = params.x + (params.width - group_width) / 2.0;
    let pet_y = params.y + (params.height - pet_height) / 2.0;
    let drew_pet = params.pet.is_some_and(|pet| {
        draw_pet(PetDrawParams {
            canvas: params.canvas,
            snapshot: params.snapshot,
            pet,
            x: pet_x,
            y: pet_y,
            size: pet_width,
            animation_time_secs: params.animation_time_secs,
            alpha: params.alpha,
        })
    });

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(Color::WHITE);
    paint.set_alpha_f(params.alpha);
    let label_x = if drew_pet {
        pet_x + pet_width + item_gap
    } else {
        params.x + (params.width - label_width) / 2.0
    };
    FontManager::global().draw_text_cached(DrawTextCachedParams {
        canvas: params.canvas,
        text: label,
        x: label_x,
        y: params.y + params.height / 2.0 + font_size * 0.35,
        size: font_size,
        bold: true,
        paint: &paint,
    });
    if drew_pet {
        draw_logo(
            params.canvas,
            label_x + label_width + item_gap,
            params.y + (params.height - logo_size) / 2.0,
            logo_size,
            params.alpha,
        );
    }
    true
}

pub fn draw_expanded(params: DrawCodexExpandedParams<'_>) -> bool {
    if params.alpha <= 0.01 {
        return false;
    }

    let header_height = expanded_header_height(params.scale, params.pet.is_some());
    let pet_width = HEADER_PET_WIDTH * params.scale;
    let pet_height = pet_display_height(pet_width);
    let pet_x = params.x + CONTENT_HORIZONTAL_PADDING * params.scale;
    let pet_y = params.y + (header_height - pet_height) / 2.0;
    let drew_pet = params.pet.is_some_and(|pet| {
        draw_pet(PetDrawParams {
            canvas: params.canvas,
            snapshot: params.snapshot,
            pet,
            x: pet_x,
            y: pet_y,
            size: pet_width,
            animation_time_secs: params.animation_time_secs,
            alpha: params.alpha,
        })
    });
    let header_height = expanded_header_height(params.scale, drew_pet);

    let mut primary_paint = Paint::default();
    primary_paint.set_anti_alias(true);
    primary_paint.set_color(Color::WHITE);
    primary_paint.set_alpha_f(params.alpha);
    let mut secondary_paint = Paint::default();
    secondary_paint.set_anti_alias(true);
    secondary_paint.set_color(Color::from_rgb(188, 191, 198));
    secondary_paint.set_alpha_f(params.alpha);

    FontManager::global().draw_text_cached(DrawTextCachedParams {
        canvas: params.canvas,
        text: "Codex",
        x: if drew_pet {
            pet_x + pet_width + HEADER_PET_TEXT_GAP * params.scale
        } else {
            pet_x
        },
        y: params.y
            + if drew_pet {
                HEADER_TITLE_Y_WITH_PET * params.scale
            } else {
                HEADER_TITLE_Y_WITHOUT_PET * params.scale
            },
        size: scaled_font_size(HEADER_TITLE_FONT_SIZE, params.font_size, params.scale),
        bold: true,
        paint: &primary_paint,
    });
    FontManager::global().draw_text_cached(DrawTextCachedParams {
        canvas: params.canvas,
        text: status_label(params.snapshot),
        x: if drew_pet {
            pet_x + pet_width + HEADER_PET_TEXT_GAP * params.scale
        } else {
            pet_x
        },
        y: params.y
            + if drew_pet {
                HEADER_STATUS_Y_WITH_PET * params.scale
            } else {
                HEADER_STATUS_Y_WITHOUT_PET * params.scale
            },
        size: scaled_font_size(HEADER_STATUS_FONT_SIZE, params.font_size, params.scale),
        bold: false,
        paint: &secondary_paint,
    });

    let body_top = params.y + header_height + CONTENT_TOP_PADDING * params.scale;
    let body_rect = Rect::from_xywh(
        params.x + CONTENT_HORIZONTAL_PADDING * params.scale,
        body_top,
        (params.width - CONTENT_HORIZONTAL_PADDING * 2.0 * params.scale).max(0.0),
        (params.height
            - header_height
            - (CONTENT_TOP_PADDING + CONTENT_BOTTOM_PADDING) * params.scale)
            .max(0.0),
    );
    let font_size = scaled_font_size(CONTENT_FONT_SIZE, params.font_size, params.scale);
    let line_height = content_line_height(params.font_size, params.scale);
    with_text_layout(params.snapshot, body_rect.width(), font_size, |lines| {
        params.canvas.save();
        params.canvas.clip_rect(body_rect, ClipOp::Intersect, true);
        let mut baseline = body_rect.top + font_size - params.scroll_offset;
        for line in lines {
            if !line.is_empty() {
                FontManager::global().draw_text_cached(DrawTextCachedParams {
                    canvas: params.canvas,
                    text: line,
                    x: body_rect.left,
                    y: baseline,
                    size: font_size,
                    bold: false,
                    paint: &primary_paint,
                });
            }
            baseline += line_height;
        }
        params.canvas.restore();

        let content_height = lines.len().max(1) as f32 * line_height;
        let scroll_max = (content_height - body_rect.height()).max(0.0);
        if scroll_max > 0.0 && body_rect.height() > 0.0 {
            let min_scrollbar_height = (18.0 * params.scale).min(body_rect.height());
            let scrollbar_height = (body_rect.height() * body_rect.height() / content_height)
                .clamp(min_scrollbar_height, body_rect.height());
            let scrollbar_y = body_rect.top
                + (body_rect.height() - scrollbar_height)
                    * (params.scroll_offset / scroll_max).clamp(0.0, 1.0);
            let mut scrollbar_paint = Paint::default();
            scrollbar_paint.set_anti_alias(true);
            scrollbar_paint.set_color(Color::from_argb(
                (params.alpha * 120.0) as u8,
                255,
                255,
                255,
            ));
            params.canvas.draw_round_rect(
                Rect::from_xywh(
                    params.x + params.width - 7.0 * params.scale,
                    scrollbar_y,
                    2.0 * params.scale,
                    scrollbar_height,
                ),
                params.scale,
                params.scale,
                &scrollbar_paint,
            );
        }
    });
    true
}

fn get_pet_image(pet: &CodexPet) -> Option<Image> {
    PET_IMAGE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let should_load = cache.as_ref().is_none_or(|cached| {
            cached.pet != *pet
                || (cached.image.is_none()
                    && cached.last_attempt.elapsed() >= PET_IMAGE_RETRY_INTERVAL)
        });
        if should_load {
            let image = match pet.read_spritesheet_bytes() {
                Ok(bytes) => decode_pet_image(&bytes),
                Err(error) => {
                    log::warn!("Could not read Codex pet '{}': {error}", pet.id);
                    None
                }
            };
            if image.is_none() {
                log::warn!(
                    "Could not decode Codex pet '{}' as a v2 spritesheet",
                    pet.id
                );
            }
            *cache = Some(CachedPetImage {
                pet: pet.clone(),
                image,
                last_attempt: Instant::now(),
            });
        }
        cache.as_ref().and_then(|cached| cached.image.clone())
    })
}

fn draw_logo(canvas: &Canvas, x: f32, y: f32, size: f32, alpha: f32) {
    let Some(image) = get_logo_image() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_alpha_f(alpha);
    canvas.draw_image_rect_with_sampling_options(
        &image,
        None,
        Rect::from_xywh(x, y, size, size),
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear),
        &paint,
    );
}

fn get_logo_image() -> Option<Image> {
    LOGO_IMAGE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let should_load = cache.as_ref().is_none_or(|cached| {
            cached.image.is_none() && cached.last_attempt.elapsed() >= LOGO_IMAGE_RETRY_INTERVAL
        });
        if should_load {
            let image = read_local_codex_logo_bytes()
                .map(|bytes| decode_logo_image(&bytes))
                .unwrap_or_else(|error| {
                    log::warn!("Could not read the local Codex logo: {error}");
                    None
                });
            if image.is_none() {
                log::warn!("Could not decode the local Codex logo");
            }
            *cache = Some(CachedLogoImage {
                image,
                last_attempt: Instant::now(),
            });
        }
        cache.as_ref().and_then(|cached| cached.image.clone())
    })
}

fn decode_logo_image(bytes: &[u8]) -> Option<Image> {
    Image::from_encoded(Data::new_copy(bytes))
}

fn decode_pet_image(bytes: &[u8]) -> Option<Image> {
    if let Some(image) = Image::from_encoded(Data::new_copy(bytes))
        .filter(|image| image.width() == ATLAS_WIDTH && image.height() == ATLAS_HEIGHT)
    {
        return Some(image);
    }

    // The Windows Skia binary used by WinIsland can omit WebP codecs. `image`
    // decodes the same local bytes, then Skia receives a standard RGBA raster.
    let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width != ATLAS_WIDTH as u32 || height != ATLAS_HEIGHT as u32 {
        return None;
    }
    let row_bytes = usize::try_from(width).ok()?.checked_mul(4)?;
    let info = ImageInfo::new(
        ISize::new(ATLAS_WIDTH, ATLAS_HEIGHT),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    skia_safe::images::raster_from_data(&info, Data::new_copy(rgba.as_raw()), row_bytes)
}

fn draw_pet(params: PetDrawParams<'_>) -> bool {
    let Some(image) = get_pet_image(params.pet) else {
        return false;
    };
    let animation = pet_animation(params.snapshot);
    let (row, frames) = animation.row_and_frames();
    let frame = frame_index(frames, params.animation_time_secs);
    let source = Rect::from_xywh(
        frame as f32 * CELL_WIDTH,
        row as f32 * CELL_HEIGHT,
        CELL_WIDTH,
        CELL_HEIGHT,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_alpha_f(params.alpha);
    params.canvas.draw_image_rect_with_sampling_options(
        &image,
        Some((&source, SrcRectConstraint::Fast)),
        Rect::from_xywh(
            params.x,
            params.y,
            params.size,
            pet_display_height(params.size),
        ),
        SamplingOptions::new(FilterMode::Nearest, MipmapMode::None),
        &paint,
    );
    true
}

const fn pet_display_height(width: f32) -> f32 {
    width * CELL_HEIGHT / CELL_WIDTH
}

fn scaled_font_size(default_size: f32, configured_size: f32, scale: f32) -> f32 {
    let base_size = if configured_size > 0.0 {
        configured_size * default_size / CONTENT_FONT_SIZE
    } else {
        default_size
    };
    base_size * scale
}

fn content_line_height(configured_size: f32, scale: f32) -> f32 {
    scaled_font_size(CONTENT_LINE_HEIGHT, configured_size, scale)
}

fn frame_index(frames: &[u32], animation_time_secs: f32) -> usize {
    let total_duration: u32 = frames.iter().sum();
    if total_duration == 0 {
        return 0;
    }
    let elapsed_ms = (animation_time_secs.max(0.0) * 1000.0) as u32 % total_duration;
    let mut accumulated = 0;
    for (index, duration) in frames.iter().enumerate() {
        accumulated += duration;
        if elapsed_ms < accumulated {
            return index;
        }
    }
    0
}

fn pet_animation(snapshot: &CodexSnapshot) -> PetAnimation {
    match snapshot.state {
        CodexState::Idle => PetAnimation::Idle,
        CodexState::Thinking => PetAnimation::Review,
        CodexState::RunningTool => PetAnimation::Running,
        CodexState::WaitingForUser | CodexState::WaitingForApproval => PetAnimation::Waiting,
        CodexState::Completed => {
            if snapshot_hash(snapshot).is_multiple_of(2) {
                PetAnimation::Waving
            } else {
                PetAnimation::Jumping
            }
        }
        CodexState::Failed => PetAnimation::Failed,
    }
}

fn status_label(snapshot: &CodexSnapshot) -> &'static str {
    let labels = match snapshot.state {
        CodexState::Idle => &["Codex 正在待命"] as &[_],
        CodexState::Thinking => &[
            "Codex 正在工作",
            "Codex 正在思考",
            "Codex 正在查找",
            "Codex 快完成了",
        ],
        CodexState::RunningTool => &[
            "Codex 正在执行",
            "Codex 正在处理",
            "Codex 正在修改",
            "Codex 正在检查",
        ],
        CodexState::WaitingForUser => &["Codex 等你决定", "Codex 需要提示", "Codex 等你选择"],
        CodexState::WaitingForApproval => &["Codex 等你确认", "Codex 等待放行", "Codex 准备好了"],
        CodexState::Completed => &[
            "Codex 完成了工作",
            "Codex 收工啦 (^_^)",
            "Codex 已交付 (^_^)",
            "Codex 做完啦 (^_^)",
        ],
        CodexState::Failed => &["Codex 遇到问题", "Codex 需要检查", "Codex 暂时卡住"],
    };
    labels[snapshot_hash(snapshot) as usize % labels.len()]
}

fn with_text_layout<T>(
    snapshot: &CodexSnapshot,
    width: f32,
    font_size: f32,
    callback: impl FnOnce(&[String]) -> T,
) -> T {
    let text = snapshot
        .latest_assistant_message
        .as_deref()
        .unwrap_or_else(|| status_label(snapshot));
    let key = text_hash(text);
    TEXT_LAYOUT.with(|cache| {
        let mut cache = cache.borrow_mut();
        let is_current = cache.as_ref().is_some_and(|layout| {
            layout.key == key
                && layout.width.to_bits() == width.to_bits()
                && layout.font_size.to_bits() == font_size.to_bits()
        });
        if !is_current {
            *cache = Some(TextLayout {
                key,
                width,
                font_size,
                lines: wrap_text(text, width, font_size),
            });
        }
        callback(&cache.as_ref().expect("text layout was initialized").lines)
    })
}

fn wrap_text(text: &str, width: f32, font_size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for logical_line in text.split('\n') {
        let logical_line = logical_line.strip_suffix('\r').unwrap_or(logical_line);
        if logical_line.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut line = String::new();
        let mut line_width = 0.0;
        let mut token = String::new();
        let mut token_is_whitespace = None;

        for grapheme in logical_line.graphemes(true) {
            let is_whitespace = grapheme.chars().all(char::is_whitespace);
            if token_is_whitespace.is_some_and(|current| current != is_whitespace) {
                append_wrapped_token(
                    &mut lines,
                    &mut line,
                    &mut line_width,
                    &token,
                    width,
                    font_size,
                );
                token.clear();
            }
            token.push_str(grapheme);
            token_is_whitespace = Some(is_whitespace);
        }

        append_wrapped_token(
            &mut lines,
            &mut line,
            &mut line_width,
            &token,
            width,
            font_size,
        );
        if !line.is_empty() {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    while lines.len() > 1 && lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

fn append_wrapped_token(
    lines: &mut Vec<String>,
    line: &mut String,
    line_width: &mut f32,
    token: &str,
    width: f32,
    font_size: f32,
) {
    if token.is_empty() {
        return;
    }

    let token_width = measure_text(token, font_size);
    if !line.is_empty() && *line_width + token_width > width - 0.5 {
        lines.push(std::mem::take(line));
        *line_width = 0.0;
    }

    if token_width <= width - 0.5 {
        line.push_str(token);
        *line_width += token_width;
        return;
    }

    for grapheme in token.graphemes(true) {
        let grapheme_width = measure_text(grapheme, font_size);
        if !line.is_empty() && *line_width + grapheme_width > width - 0.5 {
            lines.push(std::mem::take(line));
            *line_width = 0.0;
        }
        line.push_str(grapheme);
        *line_width += grapheme_width;
    }
}

fn measure_text(text: &str, font_size: f32) -> f32 {
    FontManager::global().measure_text_cached(text, font_size, FontStyle::normal())
}

fn snapshot_hash(snapshot: &CodexSnapshot) -> u64 {
    let mut hasher = DefaultHasher::new();
    snapshot.session_id.hash(&mut hasher);
    snapshot.state.hash(&mut hasher);
    hasher.finish()
}

fn text_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
