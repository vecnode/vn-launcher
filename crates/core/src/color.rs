//! A brand colour from the catalog.
//!
//! The launcher draws each app's tile from its brand colour rather than shipping an
//! icon per repository, so that adding a repository needs nothing but a hex string.
//! Contrast is computed here, in the pure core, so the "is this text readable on
//! that tile" question is answered by arithmetic rather than by eye.

use core::fmt;

/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Color {
    /// A colour from its channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse `#rrggbb` or `#rgb`, with or without the leading `#`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
        let byte = |s: &str| u8::from_str_radix(s, 16).ok();
        match hex.len() {
            6 => Some(Self::new(byte(&hex[0..2])?, byte(&hex[2..4])?, byte(&hex[4..6])?)),
            3 => {
                let expand = |s: &str| {
                    let digit = u8::from_str_radix(s, 16).ok()?;
                    Some(digit * 17)
                };
                Some(Self::new(
                    expand(&hex[0..1])?,
                    expand(&hex[1..2])?,
                    expand(&hex[2..3])?,
                ))
            }
            _ => None,
        }
    }

    /// The `#rrggbb` spelling.
    #[must_use]
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Relative luminance per ITU-R BT.709, in `0.0..=1.0`.
    #[must_use]
    pub fn luminance(self) -> f32 {
        fn linear(channel: u8) -> f32 {
            let value = f32::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }

    /// Black or white, whichever is more readable on this colour.
    ///
    /// Uses the WCAG contrast-ratio formula and picks the winner, so a pale brand
    /// colour gets dark text and a deep one gets light text.
    #[must_use]
    pub fn ink(self) -> Self {
        let luminance = self.luminance();
        let against_white = 1.05 / (luminance + 0.05);
        let against_black = (luminance + 0.05) / 0.05;
        if against_black >= against_white {
            Self::new(0x0b, 0x0d, 0x10)
        } else {
            Self::new(0xff, 0xff, 0xff)
        }
    }

    /// Blend towards `other`; `t` is clamped to `0.0..=1.0`.
    #[must_use]
    pub fn mix(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let blend = |a: u8, b: u8| {
            let value = f32::from(a) + (f32::from(b) - f32::from(a)) * t;
            value.round().clamp(0.0, 255.0) as u8
        };
        Self::new(
            blend(self.r, other.r),
            blend(self.g, other.g),
            blend(self.b, other.b),
        )
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn parses_the_spellings_a_catalog_may_use() {
        assert_eq!(Color::parse("#3b82f6"), Some(Color::new(0x3b, 0x82, 0xf6)));
        assert_eq!(Color::parse("3b82f6"), Some(Color::new(0x3b, 0x82, 0xf6)));
        assert_eq!(Color::parse("#fff"), Some(Color::new(0xff, 0xff, 0xff)));
        assert_eq!(Color::parse(" #F00 "), Some(Color::new(0xff, 0x00, 0x00)));
    }

    #[test]
    fn refuses_anything_else() {
        for bad in ["", "#", "#12", "#12345", "#gggggg", "rebeccapurple"] {
            assert_eq!(Color::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn round_trips_through_hex() {
        let color = Color::new(0x12, 0xa5, 0x8a);
        assert_eq!(color.to_hex(), "#12a58a");
        assert_eq!(Color::parse(&color.to_hex()), Some(color));
    }

    #[test]
    fn ink_is_readable_on_both_extremes() {
        // A bright tile takes dark ink; a dark tile takes light ink.
        assert_eq!(Color::new(0xf2, 0xa5, 0x16).ink(), Color::new(0x0b, 0x0d, 0x10));
        assert_eq!(Color::new(0x0b, 0x0d, 0x10).ink(), Color::new(0xff, 0xff, 0xff));
    }

    #[test]
    fn luminance_is_ordered_as_expected() {
        assert!(Color::new(0xff, 0xff, 0xff).luminance() > Color::new(0x80, 0x80, 0x80).luminance());
        assert!(Color::new(0x80, 0x80, 0x80).luminance() > Color::new(0x00, 0x00, 0x00).luminance());
    }

    #[test]
    fn mixing_reaches_both_ends() {
        let a = Color::new(0, 0, 0);
        let b = Color::new(255, 255, 255);
        assert_eq!(a.mix(b, 0.0), a);
        assert_eq!(a.mix(b, 1.0), b);
        assert_eq!(a.mix(b, 0.5), Color::new(128, 128, 128));
    }
}
