// SPDX-License-Identifier: GPL-3.0-or-later
//! A tiny built-in 5×7 bitmap font for the `--debug` on-screen overlay (HUD).
//!
//! Zero dependencies: each glyph is seven rows of five bits (bit 4 = leftmost column),
//! written as binary literals so the shapes are readable in source. Only the characters
//! the HUD needs are defined (uppercase letters, digits and a few symbols); anything else
//! renders as blank. Text is drawn directly into the window's `0x00RRGGBB` buffer, after
//! scaling, so it stays crisp and legible at any window size.

/// 5×7 glyph for `c` (ASCII, upper-cased by the caller). Unknown chars → blank.
#[rustfmt::skip]
fn glyph(c: u8) -> [u8; 7] {
    match c {
        b' ' => [0,0,0,0,0,0,0],
        b'a' => [0,0,0b01110,0b00001,0b01111,0b10001,0b01111],
        b'b' => [0b10000,0b10000,0b10110,0b11001,0b10001,0b10001,0b11110],
        b'c' => [0,0,0b01110,0b10000,0b10000,0b10001,0b01110],
        b'd' => [0b00001,0b00001,0b01101,0b10011,0b10001,0b10001,0b01111],
        b'e' => [0,0,0b01110,0b10001,0b11111,0b10000,0b01110],
        b'f' => [0b00110,0b01001,0b01000,0b11100,0b01000,0b01000,0b01000],
        b'g' => [0,0,0b01111,0b10001,0b01111,0b00001,0b01110],
        b'h' => [0b10000,0b10000,0b10110,0b11001,0b10001,0b10001,0b10001],
        b'i' => [0b00100,0,0b01100,0b00100,0b00100,0b00100,0b01110],
        b'j' => [0b00010,0,0b00110,0b00010,0b00010,0b10010,0b01100],
        b'k' => [0b10000,0b10000,0b10010,0b10100,0b11000,0b10100,0b10010],
        b'l' => [0b01100,0b00100,0b00100,0b00100,0b00100,0b00100,0b01110],
        b'm' => [0,0,0b11010,0b10101,0b10101,0b10101,0b10101],
        b'n' => [0,0,0b10110,0b11001,0b10001,0b10001,0b10001],
        b'o' => [0,0,0b01110,0b10001,0b10001,0b10001,0b01110],
        b'p' => [0,0,0b11110,0b10001,0b11110,0b10000,0b10000],
        b'q' => [0,0,0b01101,0b10011,0b01111,0b00001,0b00001],
        b'r' => [0,0,0b10110,0b11001,0b10000,0b10000,0b10000],
        b's' => [0,0,0b01111,0b10000,0b01110,0b00001,0b11110],
        b't' => [0b01000,0b01000,0b11100,0b01000,0b01000,0b01001,0b00110],
        b'u' => [0,0,0b10001,0b10001,0b10001,0b10011,0b01101],
        b'v' => [0,0,0b10001,0b10001,0b10001,0b01010,0b00100],
        b'w' => [0,0,0b10001,0b10001,0b10101,0b10101,0b01010],
        b'x' => [0,0,0b10001,0b01010,0b00100,0b01010,0b10001],
        b'y' => [0,0,0b10001,0b10001,0b01111,0b00001,0b01110],
        b'z' => [0,0,0b11111,0b00010,0b00100,0b01000,0b11111],
        b'0' => [0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110],
        b'1' => [0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110],
        b'2' => [0b01110,0b10001,0b00001,0b00010,0b00100,0b01000,0b11111],
        b'3' => [0b11111,0b00010,0b00100,0b00010,0b00001,0b10001,0b01110],
        b'4' => [0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010],
        b'5' => [0b11111,0b10000,0b11110,0b00001,0b00001,0b10001,0b01110],
        b'6' => [0b00110,0b01000,0b10000,0b11110,0b10001,0b10001,0b01110],
        b'7' => [0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000],
        b'8' => [0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110],
        b'9' => [0b01110,0b10001,0b10001,0b01111,0b00001,0b00010,0b01100],
        b'A' => [0b01110,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001],
        b'B' => [0b11110,0b10001,0b10001,0b11110,0b10001,0b10001,0b11110],
        b'C' => [0b01110,0b10001,0b10000,0b10000,0b10000,0b10001,0b01110],
        b'D' => [0b11110,0b10001,0b10001,0b10001,0b10001,0b10001,0b11110],
        b'E' => [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b11111],
        b'F' => [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b10000],
        b'G' => [0b01110,0b10001,0b10000,0b10111,0b10001,0b10001,0b01111],
        b'H' => [0b10001,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001],
        b'I' => [0b01110,0b00100,0b00100,0b00100,0b00100,0b00100,0b01110],
        b'J' => [0b00111,0b00010,0b00010,0b00010,0b00010,0b10010,0b01100],
        b'K' => [0b10001,0b10010,0b10100,0b11000,0b10100,0b10010,0b10001],
        b'L' => [0b10000,0b10000,0b10000,0b10000,0b10000,0b10000,0b11111],
        b'M' => [0b10001,0b11011,0b10101,0b10101,0b10001,0b10001,0b10001],
        b'N' => [0b10001,0b11001,0b10101,0b10011,0b10001,0b10001,0b10001],
        b'O' => [0b01110,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],
        b'P' => [0b11110,0b10001,0b10001,0b11110,0b10000,0b10000,0b10000],
        b'Q' => [0b01110,0b10001,0b10001,0b10001,0b10101,0b10010,0b01101],
        b'R' => [0b11110,0b10001,0b10001,0b11110,0b10100,0b10010,0b10001],
        b'S' => [0b01111,0b10000,0b10000,0b01110,0b00001,0b00001,0b11110],
        b'T' => [0b11111,0b00100,0b00100,0b00100,0b00100,0b00100,0b00100],
        b'U' => [0b10001,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],
        b'V' => [0b10001,0b10001,0b10001,0b10001,0b10001,0b01010,0b00100],
        b'W' => [0b10001,0b10001,0b10001,0b10101,0b10101,0b11011,0b10001],
        b'X' => [0b10001,0b10001,0b01010,0b00100,0b01010,0b10001,0b10001],
        b'Y' => [0b10001,0b10001,0b01010,0b00100,0b00100,0b00100,0b00100],
        b'Z' => [0b11111,0b00001,0b00010,0b00100,0b01000,0b10000,0b11111],
        b'.' => [0,0,0,0,0,0b00110,0b00110],
        b',' => [0,0,0,0,0b00110,0b00100,0b01000],
        b':' => [0,0b00110,0b00110,0,0b00110,0b00110,0],
        b'/' => [0b00001,0b00001,0b00010,0b00100,0b01000,0b10000,0b10000],
        b'-' => [0,0,0,0b11111,0,0,0],
        b'+' => [0,0b00100,0b00100,0b11111,0b00100,0b00100,0],
        b'#' => [0b01010,0b01010,0b11111,0b01010,0b11111,0b01010,0b01010],
        b'(' => [0b00010,0b00100,0b01000,0b01000,0b01000,0b00100,0b00010],
        b')' => [0b01000,0b00100,0b00010,0b00010,0b00010,0b00100,0b01000],
        b'%' => [0b11001,0b11010,0b00010,0b00100,0b01000,0b01011,0b10011],
        b'\'' => [0b00100,0b00100,0b01000,0,0,0,0],
        _ => [0,0,0,0,0,0,0],
    }
}

/// Pixel width of `text` rendered at `scale` (each glyph is 5 wide + 1 spacing column).
pub fn text_width(text: &str, scale: usize) -> usize {
    text.chars().count() * 6 * scale
}

/// Height of a line of text at `scale`.
pub fn line_height(scale: usize) -> usize {
    7 * scale
}

/// Fill a rectangle of the `dw`×`dh` `0x00RRGGBB` buffer with `color` (clipped).
// A drawing primitive: buffer + dims + rect + colour are all needed.
#[allow(clippy::too_many_arguments)]
pub fn fill_rect(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    for py in y..(y + h).min(dh) {
        for px in x..(x + w).min(dw) {
            buf[py * dw + px] = color;
        }
    }
}

/// Draw `text` into the `dw`×`dh` `0x00RRGGBB` buffer at `(x, y)`, `scale`× size, in
/// `color`. Characters are upper-cased; unknown ones render blank.
// A drawing primitive: buffer + dims + position + text + scale + colour are all needed.
#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    color: u32,
) {
    let mut cx = x;
    for ch in text.chars() {
        let g = glyph(ch as u8);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5usize {
                if bits & (1 << (4 - col)) != 0 {
                    let bx = cx + col * scale;
                    let by = y + row * scale;
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let (px, py) = (bx + dx, by + dy);
                            if px < dw && py < dh {
                                buf[py * dw + px] = color;
                            }
                        }
                    }
                }
            }
        }
        cx += 6 * scale;
    }
}

/// Pixel width for a fixed-width line scaled by `numerator / denominator`.
/// This permits the scene overlay's deliberate 1.5x size while keeping the
/// rest of the tiny diagnostic font on its simple integer API.
pub fn text_width_ratio(text: &str, numerator: usize, denominator: usize) -> usize {
    (text.chars().count() * 6 * numerator).div_ceil(denominator)
}

/// Height for a line scaled by `numerator / denominator`.
pub fn line_height_ratio(numerator: usize, denominator: usize) -> usize {
    (7 * numerator).div_ceil(denominator)
}

/// Draw the bitmap font at an arbitrary rational scale such as 3/2. Each
/// source pixel maps to the corresponding inclusive destination pixel range,
/// so the result remains crisp rather than becoming a blurry interpolated mess.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_ratio(
    buf: &mut [u32],
    dw: usize,
    dh: usize,
    x: usize,
    y: usize,
    text: &str,
    numerator: usize,
    denominator: usize,
    color: u32,
) {
    debug_assert!(numerator > 0 && denominator > 0);
    let mut char_index = 0usize;
    for ch in text.chars() {
        let g = glyph(ch as u8);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5usize {
                if bits & (1 << (4 - col)) == 0 {
                    continue;
                }
                let sx = (char_index * 6 + col) * numerator;
                let sy = row * numerator;
                let x0 = x + sx / denominator;
                let x1 = x + (sx + numerator).div_ceil(denominator);
                let y0 = y + sy / denominator;
                let y1 = y + (sy + numerator).div_ceil(denominator);
                for py in y0..y1.min(dh) {
                    for px in x0..x1.min(dw) {
                        buf[py * dw + px] = color;
                    }
                }
            }
        }
        char_index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_within_bounds_and_sets_pixels() {
        let (dw, dh) = (80usize, 16usize);
        let mut buf = vec![0u32; dw * dh];
        draw_text(&mut buf, dw, dh, 1, 1, "AB12", 1, 0x00FF_FFFF);
        // Something was drawn (the glyphs set some white pixels)...
        assert!(buf.contains(&0x00FF_FFFF));
        // ...and nothing leaked outside the buffer (no panic) — drawing off-edge is clipped.
        draw_text(&mut buf, dw, dh, dw - 2, dh - 2, "WW", 3, 0x00FF_FFFF);
    }

    #[test]
    #[ignore = "writes /tmp/wilson_hud.ppm for manual visual inspection of the glyphs"]
    fn render_sample_hud_ppm() {
        let (dw, dh) = (520usize, 150usize);
        let mut buf = vec![0u32; dw * dh];
        let lines = [
            "WILSON DEBUG",
            "FPS 8  FRAME 6T",
            "DAY 2/11  STAGE PLAY",
            "SCENE FISHING.ADS#1",
            "DRIFT -114,-30  ISLAND 1",
            "NIGHT 0 TIDE 0 RAFT 4",
            "FILTER XBR  SCALE FIT",
        ];
        let (scale, pad) = (2usize, 4usize);
        let lh = line_height(scale) + 2;
        for (i, l) in lines.iter().enumerate() {
            draw_text(&mut buf, dw, dh, pad, pad + i * lh, l, scale, 0x0000_FF00);
        }
        let mut out = format!("P6\n{dw} {dh}\n255\n").into_bytes();
        for &p in &buf {
            out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
        std::fs::write("/tmp/wilson_hud.ppm", out).unwrap();
    }

    #[test]
    fn unknown_chars_are_blank() {
        let (dw, dh) = (24usize, 8usize);
        let mut buf = vec![0u32; dw * dh];
        draw_text(&mut buf, dw, dh, 0, 0, "~~", 1, 0x00FF_FFFF);
        assert!(buf.iter().all(|&p| p == 0), "unknown glyphs must be blank");
    }
}
