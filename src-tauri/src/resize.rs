// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure target-dimension math. No pixels are touched here.

use crate::options::ResizeMode;

/// Largest side an ICO file can hold.
pub const ICO_MAX: u32 = 256;

/// Scale a dimension, rounding to the nearest pixel and never going below 1.
fn scale(v: u32, factor: f64) -> u32 {
    ((v as f64 * factor).round() as u32).max(1)
}

/// Scale factor to fit `src` inside `max` (0 = unconstrained) for one axis.
fn axis_factor(src: u32, max: u32) -> f64 {
    if max == 0 {
        f64::INFINITY
    } else {
        max as f64 / src as f64
    }
}

/// Compute the output size for a `src_w`×`src_h` image.
///
/// `no_upscale` guarantees neither side grows beyond the source.
pub fn target_dims(src_w: u32, src_h: u32, mode: &ResizeMode, no_upscale: bool) -> (u32, u32) {
    let (src_w, src_h) = (src_w.max(1), src_h.max(1));
    let cap = |f: f64| if no_upscale { f.min(1.0) } else { f };

    match *mode {
        ResizeMode::None => (src_w, src_h),
        ResizeMode::Percent { percent } => {
            let f = if percent.is_finite() && percent > 0.0 {
                cap(percent / 100.0)
            } else {
                1.0
            };
            (scale(src_w, f), scale(src_h, f))
        }
        ResizeMode::Fit { width, height } => {
            let f = axis_factor(src_w, width).min(axis_factor(src_h, height));
            if !f.is_finite() {
                return (src_w, src_h);
            }
            let f = cap(f);
            (scale(src_w, f), scale(src_h, f))
        }
        ResizeMode::Width { width } => {
            if width == 0 {
                return (src_w, src_h);
            }
            let f = cap(width as f64 / src_w as f64);
            (scale(src_w, f), scale(src_h, f))
        }
        ResizeMode::Height { height } => {
            if height == 0 {
                return (src_w, src_h);
            }
            let f = cap(height as f64 / src_h as f64);
            (scale(src_w, f), scale(src_h, f))
        }
        ResizeMode::Fill { width, height } => {
            let (w, h) = (
                if width == 0 { src_w } else { width },
                if height == 0 { src_h } else { height },
            );
            if no_upscale && (w > src_w || h > src_h) {
                // Shrink the crop box (keeping its aspect ratio) until it fits the source.
                let k = (src_w as f64 / w as f64).min(src_h as f64 / h as f64);
                (scale(w, k), scale(h, k))
            } else {
                (w, h)
            }
        }
        ResizeMode::Stretch { width, height } => {
            let w = if width == 0 { src_w } else { width };
            let h = if height == 0 { src_h } else { height };
            if no_upscale {
                (w.min(src_w), h.min(src_h))
            } else {
                (w, h)
            }
        }
    }
}

/// Shrink `(w, h)` so it fits inside `max`×`max`, keeping aspect ratio.
pub fn clamp_to(w: u32, h: u32, max: u32) -> (u32, u32) {
    target_dims(
        w,
        h,
        &ResizeMode::Fit {
            width: max,
            height: max,
        },
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_keeps_size() {
        assert_eq!(target_dims(640, 480, &ResizeMode::None, true), (640, 480));
    }

    #[test]
    fn percent_scales_and_respects_no_upscale() {
        let half = ResizeMode::Percent { percent: 50.0 };
        assert_eq!(target_dims(641, 480, &half, true), (321, 240));
        let double = ResizeMode::Percent { percent: 200.0 };
        assert_eq!(target_dims(100, 50, &double, false), (200, 100));
        assert_eq!(target_dims(100, 50, &double, true), (100, 50));
        // Nonsense input is ignored rather than producing 0-pixel images.
        let bad = ResizeMode::Percent { percent: -5.0 };
        assert_eq!(target_dims(100, 50, &bad, false), (100, 50));
    }

    #[test]
    fn percent_never_hits_zero() {
        let tiny = ResizeMode::Percent { percent: 1.0 };
        assert_eq!(target_dims(10, 10, &tiny, true), (1, 1));
    }

    #[test]
    fn fit_keeps_aspect() {
        let fit = ResizeMode::Fit {
            width: 1920,
            height: 1920,
        };
        assert_eq!(target_dims(4000, 3000, &fit, true), (1920, 1440));
        assert_eq!(target_dims(3000, 4000, &fit, true), (1440, 1920));
        // Small images stay small unless upscaling is allowed.
        assert_eq!(target_dims(800, 600, &fit, true), (800, 600));
        assert_eq!(target_dims(800, 600, &fit, false), (1920, 1440));
    }

    #[test]
    fn fit_with_one_unconstrained_side() {
        let fit = ResizeMode::Fit {
            width: 1000,
            height: 0,
        };
        assert_eq!(target_dims(4000, 3000, &fit, true), (1000, 750));
        let none = ResizeMode::Fit {
            width: 0,
            height: 0,
        };
        assert_eq!(target_dims(4000, 3000, &none, true), (4000, 3000));
    }

    #[test]
    fn width_and_height_modes() {
        let w = ResizeMode::Width { width: 500 };
        assert_eq!(target_dims(1000, 333, &w, true), (500, 167));
        assert_eq!(target_dims(200, 100, &w, true), (200, 100));
        assert_eq!(target_dims(200, 100, &w, false), (500, 250));
        let h = ResizeMode::Height { height: 100 };
        assert_eq!(target_dims(1000, 400, &h, true), (250, 100));
    }

    #[test]
    fn fill_is_exact_or_shrunk_box() {
        let fill = ResizeMode::Fill {
            width: 1080,
            height: 1080,
        };
        assert_eq!(target_dims(4000, 3000, &fill, true), (1080, 1080));
        // Source too small: shrink the square box to fit the short side.
        assert_eq!(target_dims(800, 600, &fill, true), (600, 600));
        assert_eq!(target_dims(800, 600, &fill, false), (1080, 1080));
    }

    #[test]
    fn stretch_ignores_aspect() {
        let s = ResizeMode::Stretch {
            width: 300,
            height: 300,
        };
        assert_eq!(target_dims(1000, 200, &s, false), (300, 300));
        assert_eq!(target_dims(1000, 200, &s, true), (300, 200));
    }

    #[test]
    fn one_pixel_sources() {
        let fit = ResizeMode::Fit {
            width: 100,
            height: 100,
        };
        assert_eq!(target_dims(1, 1, &fit, false), (100, 100));
        assert_eq!(target_dims(1, 1000, &fit, true), (1, 100));
        assert_eq!(target_dims(0, 0, &ResizeMode::None, true), (1, 1));
    }

    #[test]
    fn ico_clamp() {
        assert_eq!(clamp_to(1024, 512, ICO_MAX), (256, 128));
        assert_eq!(clamp_to(64, 64, ICO_MAX), (64, 64));
    }
}
