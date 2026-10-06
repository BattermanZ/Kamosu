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

/// The most bytes a picture may arrive as, whichever way it arrives. ADR 0033
/// sets this number for a picture Kamosu fetches from a URL, and a picture
/// handed to it directly is the same thing with a shorter journey, so
/// `web_import` reads it from here rather than keeping a second copy.
///
/// Distinct from `/api/uploads`' own 2 GB cap, which is sized for a Bundle
/// carrying a whole library. A Bundle may be enormous; one picture inside it
/// may not.
pub const MAX_PICTURE_BYTES: usize = 25 * 1024 * 1024;

/// The most pixels a picture may *claim*, checked against the header before
/// anything is decoded. A decompression bomb is small on disk and vast once
/// unpacked. 60 KB declaring 50,000 × 50,000 is 2.5 billion pixels, which is
/// several gigabytes of memory the moment it is decoded and an instance that
/// falls over. The number a file declares is the one thing available before
/// paying that cost, so it is what Kamosu decides on.
///
/// 100 megapixels is far above any camera a household owns (a 2560-pixel long
/// edge is what a Photograph is stored at in any case) and far below the size
/// at which decoding hurts.
const MAX_MEGAPIXELS: u64 = 100;

/// A Display Copy's own long edge — worked out from the Photograph, kept only
/// for convenience, and rebuildable, so these numbers are free to change
/// (ADR 0017 sets Card and Page from Aurélien's own measured library; Print
/// has no Sheet consumer yet and is a reasonable placeholder until one exists).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplaySize {
    /// A shelf card: GLOSSARY.md, "Display Copy".
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
    let mut decoder = checked_decoder(bytes)?;
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

/// Check a picture that is being kept exactly as it arrived, without remaking
/// it. A Photograph travelling in a Bundle is stored byte-for-byte, because
/// the bytes are its identity and re-encoding on receipt would report you as
/// diverged from the sender over a picture neither of you touched (ADR 0017).
/// Byte-for-byte is not the same as unexamined: what arrives still has to be
/// a picture, and still has to be one of a size Kamosu will later agree to
/// decode when it draws a Display Copy from it.
pub fn check(bytes: &[u8]) -> Result<(), OpError> {
    checked_decoder(bytes).map(|_| ())
}

/// A decoder over bytes that have passed every check Kamosu makes from a
/// header: small enough to hold, a raster format Kamosu reads, and claiming no
/// more pixels than Kamosu will unpack.
///
/// The order is the point. Each check is cheaper than the one after it, and
/// all of them happen before any pixel is decoded, which is the single place
/// an untrusted upload gets to make Kamosu do arbitrary work. Returning the
/// decoder rather than the format is what keeps that true of the caller too:
/// there is no second header parse, and no way to reach a decode that skipped
/// this function.
fn checked_decoder(bytes: &[u8]) -> Result<impl ImageDecoder + '_, OpError> {
    if bytes.len() > MAX_PICTURE_BYTES {
        return Err(OpError::bad_request(format!(
            "this picture is larger than {} MB",
            MAX_PICTURE_BYTES / (1024 * 1024)
        )));
    }
    let format = sniff(bytes)?;
    let decoder = ImageReader::with_format(Cursor::new(bytes), format)
        .into_decoder()
        .map_err(|e| OpError::bad_request(format!("cannot read this picture: {e}")))?;
    // Reading the header is all this has cost: `into_decoder` parsed the
    // dimensions out of it and no pixel data has been touched.
    let (width, height) = decoder.dimensions();
    let claimed = u64::from(width) * u64::from(height);
    if claimed > MAX_MEGAPIXELS * 1_000_000 {
        return Err(OpError::bad_request(format!(
            "this picture says it is {width} × {height}, which is more than the {MAX_MEGAPIXELS} megapixels Kamosu will open"
        )));
    }
    Ok(decoder)
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

    /// CRC-32, as PNG defines it over a chunk's type and data. Written out
    /// here rather than pulled in, because the only thing in this file that
    /// needs one is the bomb below, and a fixture that computes its own
    /// checksum cannot be wrong the way a hand-typed one could.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// A real PNG whose header *claims* an enormous picture while the file
    /// itself stays tiny, which is the shape of a decompression bomb. The pixel data
    /// is left as the small picture's, which is exactly the point: nothing
    /// may get far enough to notice it disagrees.
    fn claiming_to_be(width: u32, height: u32) -> Vec<u8> {
        let mut png = make_png(4, 4);
        // PNG is an 8-byte signature then chunks of length, type, data, CRC.
        // IHDR is always first, 13 bytes of data, and opens with the
        // dimensions, so they sit at a fixed place and the CRC follows them.
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        let crc = crc32(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        png
    }

    #[test]
    fn a_picture_claiming_more_pixels_than_kamosu_opens_is_refused_by_its_header() {
        let bomb = claiming_to_be(50_000, 50_000);
        assert!(
            bomb.len() < 1024,
            "the whole point is that it is small on disk: {} bytes",
            bomb.len()
        );
        let err = remake(&bomb).expect_err("a declared 2.5-gigapixel picture must be refused");
        assert!(
            err.message.contains("50000 × 50000"),
            "the refusal should say what the picture claimed, got: {}",
            err.message
        );
    }

    #[test]
    fn a_large_but_ordinary_picture_is_still_accepted() {
        // Comfortably under the cap, and bigger than any Photograph is kept
        // at. The check must not be a second, quieter size limit.
        remake(&make_png(5000, 4000)).expect("an ordinary large picture must still be accepted");
    }

    #[test]
    fn a_picture_larger_than_the_byte_cap_is_refused_without_being_read() {
        let too_big = vec![0u8; MAX_PICTURE_BYTES + 1];
        let err = remake(&too_big).expect_err("a picture over the byte cap must be refused");
        assert!(
            err.message.contains("25 MB"),
            "the refusal should name the cap, got: {}",
            err.message
        );
    }

    #[test]
    fn a_picture_kept_as_it_arrived_is_checked_just_the_same() {
        // The Bundle path stores bytes verbatim (ADR 0017); `check` is what
        // stops "verbatim" meaning "unexamined".
        check(&make_png(40, 30)).expect("an ordinary picture passes");
        check(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>")
            .expect_err("SVG must be refused even where nothing is re-encoded");
        check(&claiming_to_be(50_000, 50_000))
            .expect_err("a bomb must be refused even where nothing is re-encoded");
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
