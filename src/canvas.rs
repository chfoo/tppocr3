use font_kit::font::Font;
use raqote::{Color, DrawOptions, DrawTarget, Point, Source};

pub struct TextDrawer {
    fonts: [Font; 2],
    glyph_ids: [Vec<u32>; 2],
    glyph_positions: [Vec<Point>; 2],
    color: Color,
    font_size: f32, // in points,
    position: Point,
}

impl TextDrawer {
    pub fn new() -> anyhow::Result<Self> {
        let source = font_kit::source::SystemSource::new();
        let unifont = source.select_by_postscript_name("UnifontMedium")?.load()?;
        let unifont_2 = source
            .select_by_postscript_name("UnifontUpperMedium")?
            .load()?;

        Ok(Self {
            fonts: [unifont, unifont_2],
            glyph_ids: [Vec::new(), Vec::new()],
            glyph_positions: [Vec::new(), Vec::new()],
            color: Color::new(255, 255, 255, 255),
            font_size: 16.0,
            position: Point::new(0.0, 0.0),
        })
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn set_font_size(&mut self, value: f32) {
        self.font_size = value;
    }

    pub fn position(&self) -> &Point {
        &self.position
    }

    pub fn set_position(&mut self, value: Point) {
        self.position = value;
    }

    pub fn draw(&mut self, canvas: &mut DrawTarget, text: &str) {
        let mut x = self.position.x;
        let mut y = self.position.y;
        let units_per_em = self.fonts[0].metrics().units_per_em as f32;

        for character in text.chars() {
            for (index, font) in self.fonts.iter().enumerate() {
                if let Some(glyph_id) = font.glyph_for_char(character) {
                    self.glyph_ids[index].push(glyph_id);
                    self.glyph_positions[index].push(Point::new(x, y));

                    let advance = font.advance(glyph_id).unwrap();
                    x += advance.x() * self.font_size / units_per_em;
                    y += advance.y() * self.font_size / units_per_em;

                    break;
                }
            }
        }
        let source = Source::from(self.color);
        let options = DrawOptions::new();

        for (index, font) in self.fonts.iter().enumerate() {
            canvas.draw_glyphs(
                font,
                self.font_size,
                &self.glyph_ids[index],
                &self.glyph_positions[index],
                &source,
                &options,
            );

            self.glyph_ids[index].clear();
            self.glyph_positions[index].clear();
        }
    }
}
