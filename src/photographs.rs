//! A Photograph: known by its own contents, remade once at the door before it
//! has an identity (#45, ADR 0017). This module does the remaking and the
//! Display Copy work; [`crate::core::Core`] owns where the bytes live and the
//! `photographs` table that records what exists.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, imageops::FilterType};
use sha2::{Digest, Sha256};

use crate::core::OpError;

/// A Photograph's long edge, capped in pixels — the one dimension every
/// remade picture is measured against (ADR 0017). Never upscaled: a picture
/// already smaller than this is left at its own size.
pub const MAX_LONG_EDGE: u32 = 2560;

/// The quality WebP is encoded at. Chosen and measured in ADR 0017; free to
/// change later since a Photograph, once made, never changes, and only a
/// future upload is affected.
const WEBP_QUALITY: f32 = 80.0;

/// A Display Copy's own long edge — worked out from the Photograph, kept only
/// for convenience, and rebuildable, so these numbers are free to change
/// (ADR 0017 sets Card and Page from Aurélien's own measured library; Print
/// has no Sheet consumer yet and is a reasonable placeholder until one exists).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplaySize {
    /// A shelf card: CONTEXT.md, "Display Copy".
    Card,
    /// A recipe page on a phone.
    Page,
    /// A print-sized rendering for a Sheet.
    Print,
}

impl DisplaySize {
    pub fn long_edge(self) -> u32 {
        match self {
            DisplaySize::Card => 600,
            DisplaySize::Page => 1400,
            DisplaySize::Print => 2000,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DisplaySize::Card => "card",
            DisplaySize::Page => "page",
            DisplaySize::Print => "print",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "card" => Some(DisplaySize::Card),
            "page" => Some(DisplaySize::Page),
            "print" => Some(DisplaySize::Print),
            _ => None,
        }
    }
}

/// Where a Photograph's own bytes live under `/data` (ADR 0017, ADR 0028).
pub fn photographs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("photographs")
}

/// Where Display Copies live under `/data` — rebuildable, so a separate
/// directory that can be emptied without touching a Photograph.
pub fn display_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("display")
}

pub fn photograph_path(data_dir: &Path, hash: &str) -> PathBuf {
    photographs_dir(data_dir).join(format!("{hash}.webp"))
}

pub fn display_path(data_dir: &Path, hash: &str, size: DisplaySize) -> PathBuf {
    display_dir(data_dir).join(format!("{hash}-{}.webp", size.as_str()))
}

/// A Photograph's identity: the contents alone (ADR 0017), distinguished from
/// a Version's `v_` fingerprint by its own prefix.
pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("p_{}", hex::encode(Sha256::digest(bytes)))
}

/// Remake whatever arrived into the one agreed form: WebP, quality 80, at
/// most 2560 pixels on the long edge, camera metadata stripped (ADR 0017).
/// Checks the picture's own header before decoding anything, and refuses SVG
/// outright — decoding is the one place an untrusted upload gets to run
/// arbitrary-looking code, so nothing here decodes bytes it hasn't first
/// recognised as a real picture.
pub fn remake(bytes: &[u8]) -> Result<Vec<u8>, OpError> {
    let format = sniff(bytes)?;
    let mut decoder = ImageReader::with_format(Cursor::new(bytes), format)
        .into_decoder()
        .map_err(|e| OpError::bad_request(format!("cannot read this picture: {e}")))?;
    // Cameras stamp rotation into Exif rather than the pixels (eight of
    // Aurélien's own sixty do). Reading it now and baking it in before the
    // metadata is stripped below is the only chance to keep it upright.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|e| OpError::bad_request(format!("cannot read this picture: {e}")))?;
    image.apply_orientation(orientation);
    encode(&downscale(image, MAX_LONG_EDGE))
}

/// Work out one Display Copy from an already-remade Photograph. No
/// orientation handling needed: a Photograph's own bytes already carry it
/// correctly and nothing else remains in them to strip.
pub fn display_copy(photograph_bytes: &[u8], long_edge: u32) -> Result<Vec<u8>, OpError> {
    let image = image::load_from_memory_with_format(photograph_bytes, ImageFormat::WebP)
        .map_err(|e| OpError::internal(format!("cannot read stored Photograph: {e}")))?;
    encode(&downscale(image, long_edge))
}

/// Scale down to fit within `max_long_edge`, preserving aspect ratio, and
/// never upscale a picture already smaller than the cap (ADR 0017).
fn downscale(image: DynamicImage, max_long_edge: u32) -> DynamicImage {
    if image.width().max(image.height()) <= max_long_edge {
        image
    } else {
        image.resize(max_long_edge, max_long_edge, FilterType::Lanczos3)
    }
}

/// Encode as lossy WebP. Starting from a freshly decoded pixel buffer with no
/// path back to the source bytes is what strips every camera metadata field
/// as a side effect, the GPS tag above all (ADR 0017) — there is no metadata
/// chunk here to carry one.
fn encode(image: &DynamicImage) -> Result<Vec<u8>, OpError> {
    let rgb = image.to_rgb8();
    let encoder = webp::Encoder::from_rgb(rgb.as_raw(), rgb.width(), rgb.height());
    Ok(encoder.encode(WEBP_QUALITY).to_vec())
}

/// Recognise a picture by its own header before anything tries to decode it.
/// SVG is refused outright rather than folded into "unrecognised": it is XML,
/// not a raster picture, and Kamosu has no renderer for one.
fn sniff(bytes: &[u8]) -> Result<ImageFormat, OpError> {
    let sample_len = bytes.len().min(64);
    let sample = String::from_utf8_lossy(&bytes[..sample_len]);
    let trimmed = sample.trim_start();
    if trimmed.starts_with("<?xml") || trimmed.starts_with("<svg") {
        return Err(OpError::bad_request(
            "SVG is not a picture Kamosu can store — only raster formats are accepted",
        ));
    }
    image::guess_format(bytes)
        .map_err(|_| OpError::bad_request("this does not look like a picture Kamosu can read"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real, freshly encoded PNG of the given size — generated rather than
    /// hand-typed, so the fixture itself cannot be wrong the way a hand-typed
    /// CRC could be.
    fn make_png(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgb8(image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("encodes");
        bytes
    }

    #[test]
    fn remaking_the_same_picture_twice_is_byte_identical() {
        let png = make_png(40, 30);
        let a = remake(&png).expect("remakes");
        let b = remake(&png).expect("remakes");
        assert_eq!(a, b, "remaking must be deterministic for dedup to work");
        assert_eq!(hash_bytes(&a), hash_bytes(&b));
    }

    #[test]
    fn remaking_gives_a_photograph_that_decodes_as_webp() {
        let remade = remake(&make_png(40, 30)).expect("remakes");
        let decoded =
            image::load_from_memory_with_format(&remade, ImageFormat::WebP).expect("valid webp");
        assert_eq!((decoded.width(), decoded.height()), (40, 30));
    }

    #[test]
    fn a_picture_already_smaller_than_the_cap_is_never_upscaled() {
        let remade = remake(&make_png(40, 30)).expect("remakes");
        let decoded =
            image::load_from_memory_with_format(&remade, ImageFormat::WebP).expect("valid webp");
        assert_eq!((decoded.width(), decoded.height()), (40, 30));
    }

    #[test]
    fn a_picture_bigger_than_the_cap_is_downscaled_to_it() {
        let remade = remake(&make_png(4000, 2000)).expect("remakes");
        let decoded =
            image::load_from_memory_with_format(&remade, ImageFormat::WebP).expect("valid webp");
        assert_eq!(decoded.width(), MAX_LONG_EDGE);
        assert_eq!(decoded.height(), 1280);
    }

    #[test]
    fn svg_is_refused_before_any_decode_is_attempted() {
        let svg = b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let err = remake(svg).expect_err("SVG must be refused");
        assert!(err.message.to_ascii_lowercase().contains("svg"));
    }

    #[test]
    fn garbage_bytes_are_refused_rather_than_panicking() {
        let garbage = vec![0u8; 128];
        assert!(remake(&garbage).is_err());
    }

    #[test]
    fn display_size_names_round_trip() {
        for size in [DisplaySize::Card, DisplaySize::Page, DisplaySize::Print] {
            assert_eq!(DisplaySize::parse(size.as_str()), Some(size));
        }
        assert_eq!(DisplaySize::parse("thumbnail"), None);
    }
}
