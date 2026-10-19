use aws_lc_rs::rand;
use base64::{Engine, engine::general_purpose::STANDARD};

use crate::error::AppError;

/// 验证码字符集：去掉了容易混淆的 I / O / 0 / 1。
const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const LENGTH: usize = 4;

/// 5×7 点阵字形；每个字符 7 行，每行 5 列，`#` 为实心。
const GLYPHS: &[(u8, [&str; 7])] = &[
    (
        b'A',
        [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
    ),
    (
        b'B',
        [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
    ),
    (
        b'C',
        [
            ".###.", "#...#", "#....", "#....", "#....", "#...#", ".###.",
        ],
    ),
    (
        b'D',
        [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
    ),
    (
        b'E',
        [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
    ),
    (
        b'F',
        [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
    ),
    (
        b'G',
        [
            ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###.",
        ],
    ),
    (
        b'H',
        [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
    ),
    (
        b'J',
        [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
    ),
    (
        b'K',
        [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
    ),
    (
        b'L',
        [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
    ),
    (
        b'M',
        [
            "#...#", "##.##", "#.#.#", "#...#", "#...#", "#...#", "#...#",
        ],
    ),
    (
        b'N',
        [
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
    ),
    (
        b'P',
        [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
    ),
    (
        b'Q',
        [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
    ),
    (
        b'R',
        [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
    ),
    (
        b'S',
        [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
    ),
    (
        b'T',
        [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
    ),
    (
        b'U',
        [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
    ),
    (
        b'V',
        [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
    ),
    (
        b'W',
        [
            "#...#", "#...#", "#...#", "#...#", "#.#.#", "##.##", "#...#",
        ],
    ),
    (
        b'X',
        [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
    ),
    (
        b'Y',
        [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
    ),
    (
        b'Z',
        [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
    ),
    (
        b'2',
        [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
    ),
    (
        b'3',
        [
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
    ),
    (
        b'4',
        [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
    ),
    (
        b'5',
        [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
    ),
    (
        b'6',
        [
            "..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        b'7',
        [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
    ),
    (
        b'8',
        [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        b'9',
        [
            ".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##..",
        ],
    ),
];

const CELL: usize = 4;
const GLYPH_WIDTH: usize = 5 * CELL;
const GLYPH_HEIGHT: usize = 7 * CELL;
const GAP: usize = 6;
const PADDING: usize = 10;
const NOISE_DOTS: usize = 24;

/// 生成验证码明文。使用系统随机源而非可预测的时间戳片段。
pub fn random_code() -> Result<String, AppError> {
    let mut bytes = [0u8; LENGTH];
    rand::fill(&mut bytes).map_err(|_| AppError::internal("secure random generation failed"))?;

    Ok(bytes
        .iter()
        .map(|byte| CHARS[usize::from(*byte) % CHARS.len()] as char)
        .collect())
}

/// 把验证码渲染成 `data:image/svg+xml;base64,...`。
///
/// 用内联 SVG 而不是位图：不需要图像编解码依赖，体积小，前端 `<img>` 直接可用。
pub fn render_image(code: &str) -> Result<String, AppError> {
    let width = PADDING * 2 + code.len() * GLYPH_WIDTH + code.len().saturating_sub(1) * GAP;
    let height = PADDING * 2 + GLYPH_HEIGHT;
    let mut random = RandomBytes::new(64)?;

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><rect width="{width}" height="{height}" fill="{background}"/>"#,
        background = "#f4f1ea",
    );

    for index in 0..code.len() {
        let ch = code.as_bytes()[index];
        let Some((_, rows)) = GLYPHS.iter().find(|(glyph, _)| *glyph == ch) else {
            continue;
        };
        let x = PADDING + index * (GLYPH_WIDTH + GAP);
        let y = PADDING;
        let rotate = (random.next() % 17) as i32 - 8;
        let dy = (random.next() % 5) as i32 - 2;
        let color = ["#2f3542", "#1e3a5f", "#3d2c1e", "#22343c"][(random.next() % 4) as usize];
        let center_x = x as f64 + GLYPH_WIDTH as f64 / 2.0;
        let center_y = y as f64 + GLYPH_HEIGHT as f64 / 2.0;

        svg.push_str(&format!(
            r#"<g transform="rotate({rotate} {center_x} {center_y}) translate(0 {dy})" fill="{color}">"#
        ));
        for (row, line) in rows.iter().enumerate() {
            for (col, cell) in line.bytes().enumerate() {
                if cell == b'#' {
                    svg.push_str(&format!(
                        r#"<rect x="{}" y="{}" width="{CELL}" height="{CELL}"/>"#,
                        x + col * CELL,
                        y + row * CELL
                    ));
                }
            }
        }
        svg.push_str("</g>");
    }

    for _ in 0..2 {
        let x1 = random.next() % width as u32;
        let y1 = random.next() % height as u32;
        let x2 = random.next() % width as u32;
        let y2 = random.next() % height as u32;
        svg.push_str(&format!(
            r##"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#8a8f98" stroke-width="1"/>"##
        ));
    }
    for _ in 0..NOISE_DOTS {
        let x = random.next() % width as u32;
        let y = random.next() % height as u32;
        svg.push_str(&format!(
            r##"<circle cx="{x}" cy="{y}" r="1" fill="#b8bcc4"/>"##
        ));
    }
    svg.push_str("</svg>");

    Ok(format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg.as_bytes())
    ))
}

/// 一次取一批随机字节，避免每个绘制细节都调用一次系统随机源。
struct RandomBytes {
    buffer: [u8; 64],
    offset: usize,
}

impl RandomBytes {
    fn new(size: usize) -> Result<Self, AppError> {
        debug_assert!(size <= 64);
        let mut buffer = [0u8; 64];
        rand::fill(&mut buffer)
            .map_err(|_| AppError::internal("secure random generation failed"))?;

        Ok(Self { buffer, offset: 0 })
    }

    fn next(&mut self) -> u32 {
        if self.offset >= self.buffer.len() {
            self.offset = 0;
            if rand::fill(&mut self.buffer).is_err() {
                // 随机源不可用时退化为缓冲区现有内容，验证码本身仍是一次性的。
                return 0;
            }
        }
        let value = u32::from(self.buffer[self.offset]);
        self.offset += 1;

        value
    }
}

#[cfg(test)]
mod tests {
    use super::{CHARS, GLYPHS, LENGTH, random_code, render_image};

    #[test]
    fn code_uses_the_declared_alphabet() {
        let code = random_code().expect("生成验证码");

        assert_eq!(code.chars().count(), LENGTH);
        assert!(code.bytes().all(|byte| CHARS.contains(&byte)));
    }

    #[test]
    fn code_is_not_derived_from_a_timestamp() {
        let first = random_code().expect("生成验证码");
        let second = random_code().expect("生成验证码");

        assert_ne!(first, second, "两次生成的验证码不应相同");
    }

    #[test]
    fn image_is_an_inline_svg_data_uri() {
        let image = render_image("AB23").expect("渲染验证码");

        assert!(image.starts_with("data:image/svg+xml;base64,"));
        assert!(image.len() > 100);
    }

    /// 字形缺失时渲染会静默跳过该字符，图上看不到、用户必然输错。
    #[test]
    fn every_character_has_a_complete_glyph() {
        for ch in CHARS {
            let Some((_, rows)) = GLYPHS.iter().find(|(glyph, _)| glyph == ch) else {
                panic!("字符 {} 缺少字形", *ch as char);
            };
            assert_eq!(rows.len(), 7, "字符 {} 行数不对", *ch as char);
            for row in rows.iter() {
                assert_eq!(row.len(), 5, "字符 {} 的列数不对", *ch as char);
                assert!(
                    row.bytes().all(|cell| cell == b'#' || cell == b'.'),
                    "字符 {} 含非法点阵字符",
                    *ch as char
                );
            }
        }
    }

    #[test]
    fn image_never_contains_the_plaintext_code() {
        let image = render_image("AB23").expect("渲染验证码");

        assert!(!image.contains("AB23"), "图片数据不得含明文验证码");
    }
}
