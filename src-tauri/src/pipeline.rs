// SPDX-License-Identifier: GPL-3.0-or-later
//! The conversion itself: decode → orient → resize → flatten → encode → write.

use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use image::codecs::avif::AvifEncoder;
use image::codecs::bmp::BmpEncoder;
use image::codecs::gif::GifEncoder;
use image::codecs::ico::IcoEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::codecs::tiff::TiffEncoder;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, Frame, ImageDecoder, ImageError, ImageReader, RgbImage};
use rayon::prelude::*;

use crate::naming::{self, NameContext, NameReserver, Resolution};
use crate::options::{
    ConvertItem, ConvertOptions, OutputFormat, OutputPreview, PngCompression, ProgressEvent,
    ResizeMode, Summary,
};
use crate::resize::{self, ICO_MAX};
use crate::scan;

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{}", describe_image_error(.0))]
    Image(#[from] ImageError),
    #[error("{0}")]
    Other(String),
}

fn describe_image_error(e: &ImageError) -> String {
    match e {
        ImageError::Unsupported(u) => format!("Unsupported: {u}"),
        ImageError::Limits(_) => "Image is too large to process".to_string(),
        ImageError::Decoding(d) => format!("Couldn't read image: {d}"),
        ImageError::Encoding(enc) => format!("Couldn't write image: {enc}"),
        other => other.to_string(),
    }
}

type Result<T> = std::result::Result<T, ConvertError>;

/// Concrete output format and file extension for one source file.
pub fn resolve_format(requested: OutputFormat, src: &Path) -> (OutputFormat, String) {
    let concrete = |f: OutputFormat| -> (OutputFormat, String) {
        let ext = match f {
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Png | OutputFormat::Original => "png",
            OutputFormat::Webp => "webp",
            OutputFormat::Avif => "avif",
            OutputFormat::Gif => "gif",
            OutputFormat::Bmp => "bmp",
            OutputFormat::Tiff => "tiff",
            OutputFormat::Ico => "ico",
        };
        (f, ext.to_string())
    };

    if requested != OutputFormat::Original {
        return concrete(requested);
    }
    // Keep the source format and its extension spelling (`.jpeg` stays `.jpeg`).
    let ext = scan::ext_of(src);
    let format = match ext.as_str() {
        "jpg" | "jpeg" | "jpe" | "jfif" => OutputFormat::Jpeg,
        "png" => OutputFormat::Png,
        "webp" => OutputFormat::Webp,
        "gif" => OutputFormat::Gif,
        "bmp" | "dib" => OutputFormat::Bmp,
        "tif" | "tiff" => OutputFormat::Tiff,
        "ico" => OutputFormat::Ico,
        // Read-only formats (TGA, QOI, PNM, HDR, EXR) become PNG.
        _ => return concrete(OutputFormat::Png),
    };
    (format, ext)
}

/// Output size for a source of `w`×`h`, including the ICO size cap.
pub fn output_dims(w: u32, h: u32, format: OutputFormat, opts: &ConvertOptions) -> (u32, u32) {
    let (tw, th) = resize::target_dims(w, h, &opts.resize, opts.no_upscale);
    if format == OutputFormat::Ico {
        resize::clamp_to(tw, th, ICO_MAX)
    } else {
        (tw, th)
    }
}

/// Decode `path`, applying EXIF orientation so phone photos come out upright.
pub fn load(path: &Path) -> Result<DynamicImage> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(img)
}

fn resize_image(img: DynamicImage, tw: u32, th: u32, mode: &ResizeMode) -> DynamicImage {
    if img.width() == tw && img.height() == th {
        return img;
    }
    match mode {
        ResizeMode::Fill { .. } => img.resize_to_fill(tw, th, FilterType::Lanczos3),
        _ => img.resize_exact(tw, th, FilterType::Lanczos3),
    }
}

/// `#rrggbb` → `[r, g, b]`, white if it can't be parsed.
pub fn parse_hex_color(s: &str) -> [u8; 3] {
    let hex = s.trim().trim_start_matches('#');
    let parse = |i: usize| {
        hex.get(i..i + 2)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
    };
    match (hex.len(), parse(0), parse(2), parse(4)) {
        (6, Some(r), Some(g), Some(b)) => [r, g, b],
        _ => [255, 255, 255],
    }
}

/// Composite onto a solid background (for formats without transparency).
fn flatten(img: &DynamicImage, bg: [u8; 3]) -> RgbImage {
    let rgba = img.to_rgba8();
    let mut out = RgbImage::new(rgba.width(), rgba.height());
    for (dst, src) in out.pixels_mut().zip(rgba.pixels()) {
        let a = src[3] as u32;
        for c in 0..3 {
            dst[c] = ((src[c] as u32 * a + bg[c] as u32 * (255 - a) + 127) / 255) as u8;
        }
    }
    out
}

/// Reduce to 8 bits per channel, keeping grey/alpha channels where possible.
fn to_8bit(img: DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageLuma8(_)
        | DynamicImage::ImageLumaA8(_)
        | DynamicImage::ImageRgb8(_)
        | DynamicImage::ImageRgba8(_) => img,
        DynamicImage::ImageLuma16(_) => DynamicImage::ImageLuma8(img.to_luma8()),
        DynamicImage::ImageLumaA16(_) => DynamicImage::ImageLumaA8(img.to_luma_alpha8()),
        _ if img.color().has_alpha() => DynamicImage::ImageRgba8(img.to_rgba8()),
        _ => DynamicImage::ImageRgb8(img.to_rgb8()),
    }
}

/// 8-bit RGB or RGBA (for encoders that don't take grey images).
fn to_rgb_or_rgba8(img: DynamicImage) -> DynamicImage {
    if img.color().has_alpha() {
        DynamicImage::ImageRgba8(img.to_rgba8())
    } else {
        DynamicImage::ImageRgb8(img.to_rgb8())
    }
}

/// PNG and TIFF can store 16-bit; keep it, but turn float images into 16-bit.
fn keep_deep(img: DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageRgb32F(_) => DynamicImage::ImageRgb16(img.to_rgb16()),
        DynamicImage::ImageRgba32F(_) => DynamicImage::ImageRgba16(img.to_rgba16()),
        _ => img,
    }
}

/// Encode `img` into memory in the requested format.
pub fn encode(img: DynamicImage, format: OutputFormat, opts: &ConvertOptions) -> Result<Vec<u8>> {
    let quality = opts.quality.clamp(1, 100);
    let mut buf = Vec::new();

    match format {
        OutputFormat::Jpeg => {
            let img = match img {
                DynamicImage::ImageLuma8(_) | DynamicImage::ImageLuma16(_) => {
                    DynamicImage::ImageLuma8(img.to_luma8())
                }
                _ if img.color().has_alpha() => {
                    DynamicImage::ImageRgb8(flatten(&img, parse_hex_color(&opts.background)))
                }
                _ => DynamicImage::ImageRgb8(img.to_rgb8()),
            };
            img.write_with_encoder(JpegEncoder::new_with_quality(&mut buf, quality))?;
        }
        OutputFormat::Png | OutputFormat::Original => {
            let compression = match opts.png_compression {
                PngCompression::Fast => CompressionType::Fast,
                PngCompression::Default => CompressionType::Default,
                PngCompression::Best => CompressionType::Best,
            };
            keep_deep(img).write_with_encoder(PngEncoder::new_with_quality(
                &mut buf,
                compression,
                PngFilter::Adaptive,
            ))?;
        }
        OutputFormat::Webp => {
            let img = to_rgb_or_rgba8(img);
            if opts.webp_lossless {
                img.write_with_encoder(WebPEncoder::new_lossless(&mut buf))?;
            } else {
                let (w, h) = (img.width(), img.height());
                let encoder = match &img {
                    DynamicImage::ImageRgba8(rgba) => webp::Encoder::from_rgba(rgba, w, h),
                    DynamicImage::ImageRgb8(rgb) => webp::Encoder::from_rgb(rgb, w, h),
                    _ => unreachable!("converted to RGB(A)8 above"),
                };
                let mem = encoder
                    .encode_simple(false, quality as f32)
                    .map_err(|e| ConvertError::Other(format!("WebP encoding failed: {e:?}")))?;
                buf.extend_from_slice(&mem);
            }
        }
        OutputFormat::Avif => {
            // Speed 6 of 1..=10: good compression without taking minutes per photo.
            to_rgb_or_rgba8(img)
                .write_with_encoder(AvifEncoder::new_with_speed_quality(&mut buf, 6, quality))?;
        }
        OutputFormat::Gif => {
            let mut encoder = GifEncoder::new_with_speed(&mut buf, 10);
            encoder.encode_frame(Frame::new(img.to_rgba8()))?;
        }
        OutputFormat::Bmp => {
            to_8bit(img).write_with_encoder(BmpEncoder::new(&mut buf))?;
        }
        OutputFormat::Tiff => {
            keep_deep(img).write_with_encoder(TiffEncoder::new(Cursor::new(&mut buf)))?;
        }
        OutputFormat::Ico => {
            DynamicImage::ImageRgba8(img.to_rgba8())
                .write_with_encoder(IcoEncoder::new(&mut buf))?;
        }
    }
    Ok(buf)
}

/// Write via a temporary file in the same folder, then move it into place,
/// so a crash or cancel never leaves a half-written image behind.
fn write_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".to_string());
    let tmp = target.with_file_name(format!(".{file_name}.morphosis-tmp"));
    if let Err(e) = fs::write(&tmp, bytes).and_then(|_| fs::rename(&tmp, target)) {
        let _ = fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

/// What happened to one file.
pub enum Outcome {
    Done {
        output: PathBuf,
        in_bytes: u64,
        out_bytes: u64,
        width: u32,
        height: u32,
    },
    Skipped(String),
}

/// Fixed per-batch data.
pub struct Batch<'a> {
    pub opts: &'a ConvertOptions,
    pub date: String,
    pub reserver: NameReserver,
}

impl<'a> Batch<'a> {
    pub fn new(opts: &'a ConvertOptions) -> Self {
        Self {
            opts,
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            reserver: NameReserver::new(),
        }
    }

    /// Desired output path (before conflict handling) and output size.
    fn plan(&self, item: &ConvertItem, index: usize) -> Result<(PathBuf, OutputFormat, u32, u32)> {
        let src = Path::new(&item.path);
        let (format, ext) = resolve_format(self.opts.format, src);
        let (w, h) = scan::probe(src).map_err(ConvertError::Other)?;
        let (tw, th) = output_dims(w, h, format, self.opts);

        let stem = src
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let rendered = naming::render_template(
            &self.opts.output.name_template,
            &NameContext {
                stem: &stem,
                width: tw,
                height: th,
                format: &ext,
                date: &self.date,
                index,
            },
        );
        let file_name = format!("{}.{ext}", naming::sanitize_stem(&rendered, &stem));
        let dir = naming::output_dir(src, &item.rel_dir, &self.opts.output);
        Ok((dir.join(file_name), format, tw, th))
    }

    pub fn process(&self, item: &ConvertItem, index: usize) -> Result<Outcome> {
        let src = Path::new(&item.path);
        let (desired, format, tw, th) = self.plan(item, index)?;

        // Resolve the name before decoding so "Skip" is instant on re-runs.
        let target = match self
            .reserver
            .resolve(&desired, src, self.opts.output.conflict)
        {
            Resolution::Write(p) => p,
            Resolution::Skip(reason) => return Ok(Outcome::Skipped(reason)),
        };

        let in_bytes = fs::metadata(src)?.len();
        let img = load(src)?;
        let img = resize_image(img, tw, th, &self.opts.resize);
        let (width, height) = (img.width(), img.height());
        let bytes = encode(img, format, self.opts)?;

        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&target, &bytes)?;

        Ok(Outcome::Done {
            output: target,
            in_bytes,
            out_bytes: bytes.len() as u64,
            width,
            height,
        })
    }

    pub fn preview(&self, item: &ConvertItem) -> Result<OutputPreview> {
        let (path, _, width, height) = self.plan(item, 1)?;
        Ok(OutputPreview {
            exists: path.exists(),
            is_source: naming::is_same_file(&path, Path::new(&item.path)),
            path: path.to_string_lossy().to_string(),
            width,
            height,
        })
    }
}

/// Worker threads: leave one core free so the UI stays responsive.
fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1))
        .unwrap_or(1)
        .max(1)
}

/// Convert all `items`, reporting through `emit`. Stops early when `cancel` is set.
pub fn run_batch(
    items: Vec<ConvertItem>,
    opts: &ConvertOptions,
    cancel: &AtomicBool,
    emit: impl Fn(ProgressEvent) + Sync,
) -> Summary {
    let started = Instant::now();
    let total = items.len();
    emit(ProgressEvent::Started { total });

    let batch = Batch::new(opts);
    let converted = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let in_total = AtomicU64::new(0);
    let out_total = AtomicU64::new(0);
    let dirs = Mutex::new(BTreeSet::new());

    let work = || {
        // `par_bridge` keeps the processing order close to the list order.
        items
            .into_iter()
            .enumerate()
            .par_bridge()
            .for_each(|(i, item)| {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                let path = item.path.clone();
                emit(ProgressEvent::FileStarted { path: path.clone() });
                match batch.process(&item, i + 1) {
                    Ok(Outcome::Done {
                        output,
                        in_bytes,
                        out_bytes,
                        width,
                        height,
                    }) => {
                        converted.fetch_add(1, Ordering::Relaxed);
                        in_total.fetch_add(in_bytes, Ordering::Relaxed);
                        out_total.fetch_add(out_bytes, Ordering::Relaxed);
                        if let Some(dir) = output.parent() {
                            dirs.lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .insert(dir.to_string_lossy().to_string());
                        }
                        emit(ProgressEvent::FileDone {
                            path,
                            output_path: output.to_string_lossy().to_string(),
                            out_bytes,
                            width,
                            height,
                        });
                    }
                    Ok(Outcome::Skipped(reason)) => {
                        skipped.fetch_add(1, Ordering::Relaxed);
                        emit(ProgressEvent::FileSkipped { path, reason });
                    }
                    Err(e) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                        emit(ProgressEvent::FileFailed {
                            path,
                            error: e.to_string(),
                        });
                    }
                }
            });
    };

    match rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count())
        .build()
    {
        Ok(pool) => pool.install(work),
        Err(_) => work(),
    }

    let (converted, skipped, failed) = (
        converted.into_inner(),
        skipped.into_inner(),
        failed.into_inner(),
    );
    let summary = Summary {
        converted,
        skipped,
        failed,
        // Only "cancelled" if cancelling actually left files untouched.
        cancelled: cancel.load(Ordering::Relaxed) && converted + skipped + failed < total,
        in_bytes: in_total.into_inner(),
        out_bytes: out_total.into_inner(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        output_dirs: dirs
            .into_inner()
            .unwrap_or_else(|e| e.into_inner())
            .into_iter()
            .collect(),
    };
    emit(ProgressEvent::Finished {
        summary: summary.clone(),
    });
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{ConflictPolicy, OutputOptions};
    use image::{ImageFormat, Rgba, RgbaImage};

    fn opts(format: OutputFormat, dir: &Path) -> ConvertOptions {
        ConvertOptions {
            format,
            quality: 80,
            webp_lossless: false,
            png_compression: PngCompression::Default,
            background: "#ff0000".into(),
            resize: ResizeMode::Fit {
                width: 300,
                height: 300,
            },
            no_upscale: true,
            output: OutputOptions {
                directory: Some(dir.to_string_lossy().to_string()),
                keep_structure: false,
                name_template: "{name}".into(),
                conflict: ConflictPolicy::Rename,
            },
        }
    }

    /// 600×400 image whose left half is fully transparent.
    fn sample(dir: &Path) -> PathBuf {
        let img = RgbaImage::from_fn(600, 400, |x, _| {
            if x < 300 {
                Rgba([0, 0, 255, 0])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let p = dir.join("sample.png");
        img.save(&p).unwrap();
        p
    }

    fn item(p: &Path) -> ConvertItem {
        ConvertItem {
            path: p.to_string_lossy().to_string(),
            rel_dir: String::new(),
        }
    }

    #[test]
    fn converts_to_every_format() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let out = dir.path().join("out");

        let cases = [
            (OutputFormat::Jpeg, ImageFormat::Jpeg, "jpg", (300, 200)),
            (OutputFormat::Png, ImageFormat::Png, "png", (300, 200)),
            (OutputFormat::Webp, ImageFormat::WebP, "webp", (300, 200)),
            (OutputFormat::Avif, ImageFormat::Avif, "avif", (300, 200)),
            (OutputFormat::Gif, ImageFormat::Gif, "gif", (300, 200)),
            (OutputFormat::Bmp, ImageFormat::Bmp, "bmp", (300, 200)),
            (OutputFormat::Tiff, ImageFormat::Tiff, "tiff", (300, 200)),
            (OutputFormat::Ico, ImageFormat::Ico, "ico", (256, 171)),
        ];
        for (fmt, expected, ext, dims) in cases {
            let mut o = opts(fmt, &out);
            if fmt == OutputFormat::Ico {
                o.resize = ResizeMode::None;
            }
            let batch = Batch::new(&o);
            let Outcome::Done {
                output,
                width,
                height,
                ..
            } = batch.process(&item(&src), 1).unwrap()
            else {
                panic!("{fmt:?} was skipped");
            };
            assert_eq!(output, out.join(format!("sample.{ext}")), "{fmt:?}");
            assert_eq!((width, height), dims, "{fmt:?}");

            let bytes = fs::read(&output).unwrap();
            assert_eq!(image::guess_format(&bytes).unwrap(), expected, "{fmt:?}");
            // AVIF can't be decoded without dav1d; everything else round-trips.
            if fmt != OutputFormat::Avif {
                let back = image::load_from_memory(&bytes).unwrap();
                assert_eq!((back.width(), back.height()), dims, "{fmt:?}");
            }
        }
    }

    #[test]
    fn jpeg_flattens_onto_background() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let o = opts(OutputFormat::Jpeg, dir.path());
        let Outcome::Done { output, .. } = Batch::new(&o).process(&item(&src), 1).unwrap() else {
            panic!("skipped");
        };
        let back = image::open(output).unwrap().to_rgb8();
        let left = back.get_pixel(10, 100);
        let right = back.get_pixel(290, 100);
        assert!(
            left[0] > 200 && left[2] < 60,
            "transparent → red bg: {left:?}"
        );
        assert!(
            right[2] > 200 && right[0] < 60,
            "opaque stays blue: {right:?}"
        );
    }

    #[test]
    fn avif_keeps_alpha() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let o = opts(OutputFormat::Avif, dir.path());
        let Outcome::Done { output, .. } = Batch::new(&o).process(&item(&src), 1).unwrap() else {
            panic!("skipped");
        };
        // We can't decode AVIF, but an alpha plane is stored as an auxiliary image
        // tagged with this URN in the container.
        let bytes = fs::read(output).unwrap();
        let urn = b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha";
        assert!(
            bytes.windows(urn.len()).any(|w| w == urn),
            "AVIF output has no alpha plane"
        );
    }

    #[test]
    fn webp_lossless_and_original_format() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let mut o = opts(OutputFormat::Webp, &dir.path().join("o"));
        o.webp_lossless = true;
        o.resize = ResizeMode::None;
        let Outcome::Done { output, .. } = Batch::new(&o).process(&item(&src), 1).unwrap() else {
            panic!("skipped");
        };
        let back = image::open(output).unwrap().to_rgba8();
        assert_eq!(back.get_pixel(10, 10)[3], 0, "alpha survives lossless WebP");
        assert_eq!(back.get_pixel(500, 10), &Rgba([0, 0, 255, 255]));

        assert_eq!(
            resolve_format(OutputFormat::Original, Path::new("a.JPEG")),
            (OutputFormat::Jpeg, "jpeg".to_string())
        );
        assert_eq!(
            resolve_format(OutputFormat::Original, Path::new("a.tga")),
            (OutputFormat::Png, "png".to_string())
        );
    }

    #[test]
    fn source_is_never_overwritten_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let before = fs::read(&src).unwrap();
        let mut o = opts(OutputFormat::Png, dir.path());
        o.output.directory = None; // same folder as the original
        o.output.conflict = ConflictPolicy::Overwrite;
        let Outcome::Done { output, .. } = Batch::new(&o).process(&item(&src), 1).unwrap() else {
            panic!("skipped");
        };
        assert_eq!(output, dir.path().join("sample (1).png"));
        assert_eq!(fs::read(&src).unwrap(), before);
    }

    #[test]
    fn batch_reports_progress_and_cancel() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let missing = dir.path().join("missing.png");
        let o = opts(OutputFormat::Png, &dir.path().join("out"));
        let events = Mutex::new(Vec::new());

        let summary = run_batch(
            vec![item(&src), item(&missing)],
            &o,
            &AtomicBool::new(false),
            |e| events.lock().unwrap().push(e),
        );
        assert_eq!((summary.converted, summary.failed), (1, 1));
        assert!(!summary.cancelled);
        // started, 2 × (fileStarted + result), finished
        assert_eq!(events.lock().unwrap().len(), 6);

        let cancelled = run_batch(vec![item(&src)], &o, &AtomicBool::new(true), |_| {});
        assert!(cancelled.cancelled);
        assert_eq!(cancelled.converted, 0);
    }

    #[test]
    fn exif_orientation_swaps_dimensions() {
        assert!(scan::swaps_axes(Orientation::Rotate90));
        assert!(!scan::swaps_axes(Orientation::FlipHorizontal));
        let mut img = DynamicImage::ImageRgb8(RgbImage::new(40, 20));
        img.apply_orientation(Orientation::Rotate90);
        assert_eq!((img.width(), img.height()), (20, 40));
    }

    #[test]
    fn hex_colors() {
        assert_eq!(parse_hex_color("#1a2B3c"), [0x1a, 0x2b, 0x3c]);
        assert_eq!(parse_hex_color("nope"), [255, 255, 255]);
    }
}
