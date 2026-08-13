use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageReader, Limits};
use skia_safe::{AlphaType, ColorType, Data, Image, ImageInfo, images};

const MAX_SOURCE_DIMENSION: u32 = 8192;
const MAX_SOURCE_PIXELS: u64 = 8 * 1024 * 1024;
const MAX_DECODE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_OUTPUT_DIMENSION: u32 = 1024;

pub(crate) fn decode_cover_image(data: &Data) -> Option<Image> {
    let mut reader = ImageReader::new(Cursor::new(data.as_bytes()))
        .with_guessed_format()
        .ok()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
    limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);

    let decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    let pixel_count = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || pixel_count > MAX_SOURCE_PIXELS
        || decoder.total_bytes() > MAX_DECODE_BYTES
    {
        return None;
    }

    let decoded = DynamicImage::from_decoder(decoder).ok()?;
    let decoded = if width.max(height) > MAX_OUTPUT_DIMENSION {
        decoded.thumbnail(MAX_OUTPUT_DIMENSION, MAX_OUTPUT_DIMENSION)
    } else {
        decoded
    };
    let rgba = decoded.into_rgba8();
    let (width, height) = rgba.dimensions();
    let info = ImageInfo::new(
        (i32::try_from(width).ok()?, i32::try_from(height).ok()?),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    images::raster_from_data(&info, Data::new_copy(rgba.as_raw()), info.min_row_bytes())
}
