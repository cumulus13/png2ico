//! Minimal RGBA color parsing for `--background`.

use crate::error::Error;
use std::str::FromStr;

/// An 8-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub [u8; 4]);

impl Color {
    pub const TRANSPARENT: Color = Color([0, 0, 0, 0]);
}

impl FromStr for Color {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || Error::InvalidColor(s.to_string());
        let t = s.trim();
        match t.to_ascii_lowercase().as_str() {
            "transparent" | "none" => return Ok(Color::TRANSPARENT),
            "white" => return Ok(Color([255, 255, 255, 255])),
            "black" => return Ok(Color([0, 0, 0, 255])),
            _ => {}
        }

        let hex = t.strip_prefix('#').unwrap_or(t);
        if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad());
        }

        // All chars are ASCII hex digits, so byte slicing is safe.
        let nibble = |i: usize| u8::from_str_radix(&hex[i..=i], 16).map(|v| v * 17).map_err(|_| bad());
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| bad());

        let rgba = match hex.len() {
            3 => [nibble(0)?, nibble(1)?, nibble(2)?, 255],
            4 => [nibble(0)?, nibble(1)?, nibble(2)?, nibble(3)?],
            6 => [byte(0)?, byte(2)?, byte(4)?, 255],
            8 => [byte(0)?, byte(2)?, byte(4)?, byte(6)?],
            _ => return Err(bad()),
        };
        Ok(Color(rgba))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_formats() {
        assert_eq!("#fff".parse::<Color>().unwrap(), Color([255, 255, 255, 255]));
        assert_eq!("#ff000080".parse::<Color>().unwrap(), Color([255, 0, 0, 128]));
        assert_eq!("00ff00".parse::<Color>().unwrap(), Color([0, 255, 0, 255]));
        assert_eq!("transparent".parse::<Color>().unwrap(), Color::TRANSPARENT);
    }

    #[test]
    fn rejects_garbage() {
        assert!("#12".parse::<Color>().is_err());
        assert!("#gggggg".parse::<Color>().is_err());
        assert!("".parse::<Color>().is_err());
    }
}
