//! Conversions between the CIE xy / mirek values the bridge speaks and the
//! RGB / HSV values the UI draws with.

/// An sRGB color with components in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Rgb {
    pub fn to_hex(self) -> String {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b))
    }
}

fn srgb_to_linear(v: f64) -> f64 {
    if v > 0.04045 {
        ((v + 0.055) / 1.055).powf(2.4)
    } else {
        v / 12.92
    }
}

fn linear_to_srgb(v: f64) -> f64 {
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Converts an sRGB color to CIE xy, using the wide-gamut matrix from the Hue
/// developer documentation. The bridge clamps the result to each lamp's gamut.
pub fn rgb_to_xy(rgb: Rgb) -> (f64, f64) {
    let r = srgb_to_linear(rgb.r);
    let g = srgb_to_linear(rgb.g);
    let b = srgb_to_linear(rgb.b);
    let x = r * 0.664_511 + g * 0.154_324 + b * 0.162_028;
    let y = r * 0.283_881 + g * 0.668_433 + b * 0.047_685;
    let z = r * 0.000_088 + g * 0.072_310 + b * 0.986_039;
    let sum = x + y + z;
    if sum <= f64::EPSILON {
        // Black has no chromaticity; fall back to the D65 white point.
        return (0.3127, 0.3290);
    }
    (x / sum, y / sum)
}

/// Converts CIE xy to the brightest sRGB color with that chromaticity.
pub fn xy_to_rgb(x: f64, y: f64) -> Rgb {
    let y = y.max(1e-6);
    let big_y = 1.0;
    let big_x = big_y / y * x;
    let big_z = big_y / y * (1.0 - x - y);
    let mut r = big_x * 1.656_492 - big_y * 0.354_851 - big_z * 0.255_038;
    let mut g = -big_x * 0.707_196 + big_y * 1.655_397 + big_z * 0.036_152;
    let mut b = big_x * 0.051_713 - big_y * 0.121_364 + big_z * 1.011_530;
    // Out-of-gamut colors produce negative components; clip, then normalise so
    // the strongest channel is at full intensity.
    r = r.max(0.0);
    g = g.max(0.0);
    b = b.max(0.0);
    let max = r.max(g).max(b);
    if max > 0.0 {
        r /= max;
        g /= max;
        b /= max;
    }
    Rgb {
        r: linear_to_srgb(r).clamp(0.0, 1.0),
        g: linear_to_srgb(g).clamp(0.0, 1.0),
        b: linear_to_srgb(b).clamp(0.0, 1.0),
    }
}

/// Hue in degrees `0..360`, saturation and value in `0..=1`.
pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> Rgb {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Rgb {
        r: r + m,
        g: g + m,
        b: b + m,
    }
}

/// Returns `(hue_degrees, saturation, value)`.
pub fn rgb_to_hsv(rgb: Rgb) -> (f64, f64, f64) {
    let max = rgb.r.max(rgb.g).max(rgb.b);
    let min = rgb.r.min(rgb.g).min(rgb.b);
    let d = max - min;
    let h = if d <= f64::EPSILON {
        0.0
    } else if max == rgb.r {
        60.0 * ((rgb.g - rgb.b) / d).rem_euclid(6.0)
    } else if max == rgb.g {
        60.0 * ((rgb.b - rgb.r) / d + 2.0)
    } else {
        60.0 * ((rgb.r - rgb.g) / d + 4.0)
    };
    let s = if max <= f64::EPSILON { 0.0 } else { d / max };
    (h, s, max)
}

/// Approximates the color of a white light at the given mirek
/// (Tanner Helland's black-body fit), for drawing swatches.
pub fn mirek_to_rgb(mirek: f64) -> Rgb {
    let t = (1_000_000.0 / mirek.max(1.0)) / 100.0;
    let r = if t <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2)
    };
    let g = if t <= 66.0 {
        99.470_802_586_1 * t.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    };
    Rgb {
        r: (r / 255.0).clamp(0.0, 1.0),
        g: (g / 255.0).clamp(0.0, 1.0),
        b: (b / 255.0).clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn hex_formatting() {
        assert_eq!(
            Rgb {
                r: 1.0,
                g: 0.5,
                b: 0.0
            }
            .to_hex(),
            "#ff8000"
        );
    }

    #[test]
    fn primaries_round_trip_through_xy() {
        for rgb in [
            Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.0,
            },
            Rgb {
                r: 0.0,
                g: 1.0,
                b: 0.0,
            },
            Rgb {
                r: 0.0,
                g: 0.0,
                b: 1.0,
            },
            Rgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            Rgb {
                r: 1.0,
                g: 0.6,
                b: 0.2,
            },
        ] {
            let (x, y) = rgb_to_xy(rgb);
            let back = xy_to_rgb(x, y);
            assert!(close(back.r, rgb.r, 0.02), "{rgb:?} -> {back:?}");
            assert!(close(back.g, rgb.g, 0.02), "{rgb:?} -> {back:?}");
            assert!(close(back.b, rgb.b, 0.02), "{rgb:?} -> {back:?}");
        }
    }

    #[test]
    fn white_is_near_d65() {
        let (x, y) = rgb_to_xy(Rgb {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        });
        assert!(close(x, 0.3227, 0.01) && close(y, 0.329, 0.01), "{x} {y}");
    }

    #[test]
    fn hsv_round_trip() {
        for (h, s) in [(0.0, 1.0), (120.0, 0.5), (240.0, 0.25), (300.0, 0.9)] {
            let (h2, s2, v2) = rgb_to_hsv(hsv_to_rgb(h, s, 1.0));
            assert!(close(h, h2, 0.5) && close(s, s2, 0.01) && close(v2, 1.0, 1e-9));
        }
    }

    #[test]
    fn warm_white_is_warmer_than_cool_white() {
        let warm = mirek_to_rgb(500.0);
        let cool = mirek_to_rgb(153.0);
        assert!(warm.b < cool.b);
        assert!(warm.r >= cool.r);
    }
}
