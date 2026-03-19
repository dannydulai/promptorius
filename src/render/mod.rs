//! ANSI escape sequence generation and unicode width calculation.
//!
//! This is a leaf module — it knows about terminal output but nothing about
//! Rhai, config, or the pipeline.

use crate::config::ColorDef;
use nu_ansi_term::{Color, Style};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("unknown color: {0}")]
    UnknownColor(String),
}

/// Convert a ColorDef from config into an ANSI escape sequence string.
pub fn color_to_ansi(def: &ColorDef) -> String {
    let style = color_def_to_style(def);
    style.prefix().to_string()
}

/// Return the ANSI reset sequence.
pub fn ansi_reset() -> String {
    "\x1b[0m".to_string()
}

fn color_def_to_style(def: &ColorDef) -> Style {
    match def {
        ColorDef::Simple(name) => {
            let color = parse_color(name);
            Style::new().fg(color)
        }
        ColorDef::Full {
            fg,
            bg,
            bold,
            italic,
            dim,
            strikethrough,
            underline,
            underline_color: _,
        } => {
            let mut style = Style::new();
            if let Some(fg) = fg {
                style = style.fg(parse_color(fg));
            }
            if let Some(bg) = bg {
                style = style.on(parse_color(bg));
            }
            if *bold {
                style = style.bold();
            }
            if *italic {
                style = style.italic();
            }
            if *dim {
                style = style.dimmed();
            }
            if *strikethrough {
                style = style.strikethrough();
            }
            if let Some(underline_style) = underline {
                // Basic underline — kitty extended styles would need raw escape codes
                let _ = underline_style;
                style = style.underline();
            }
            style
        }
    }
}

/// Parse a color string into a nu_ansi_term Color.
/// Supports: named colors, bright_* variants, #RGB, #RRGGBB, 256-palette numbers.
fn parse_color(s: &str) -> Color {
    // Hex colors
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    // 256-palette number
    if let Ok(n) = s.parse::<u8>() {
        return Color::Fixed(n);
    }

    // Named colors
    match s {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "purple" | "magenta" => Color::Purple,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "bright_black" => Color::DarkGray,
        "bright_red" => Color::LightRed,
        "bright_green" => Color::LightGreen,
        "bright_yellow" => Color::LightYellow,
        "bright_blue" => Color::LightBlue,
        "bright_purple" | "bright_magenta" => Color::LightPurple,
        "bright_cyan" => Color::LightCyan,
        "bright_white" => Color::LightGray,
        _ => Color::White, // fallback
    }
}

fn parse_hex_color(hex: &str) -> Color {
    match hex.len() {
        // #RGB -> expand to #RRGGBB
        3 => {
            let chars: Vec<char> = hex.chars().collect();
            let r = hex_char_to_u8(chars[0]) * 17;
            let g = hex_char_to_u8(chars[1]) * 17;
            let b = hex_char_to_u8(chars[2]) * 17;
            Color::Rgb(r, g, b)
        }
        // #RRGGBB
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            Color::Rgb(r, g, b)
        }
        _ => Color::White,
    }
}

fn hex_char_to_u8(c: char) -> u8 {
    match c {
        '0'..='9' => c as u8 - b'0',
        'a'..='f' => c as u8 - b'a' + 10,
        'A'..='F' => c as u8 - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_color() {
        assert!(matches!(parse_color("red"), Color::Red));
        assert!(matches!(parse_color("bright_cyan"), Color::LightCyan));
    }

    #[test]
    fn parse_hex_short() {
        assert!(matches!(parse_color("#f00"), Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn parse_hex_long() {
        assert!(matches!(parse_color("#ff5f00"), Color::Rgb(255, 95, 0)));
    }

    #[test]
    fn parse_256_palette() {
        assert!(matches!(parse_color("196"), Color::Fixed(196)));
    }

    #[test]
    fn reset_is_nonempty() {
        let reset = ansi_reset();
        assert!(!reset.is_empty());
    }
}
