use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use eddie::JaroWinkler;

use crate::{config::Region, text_recognizer::BoundingBox};

pub trait TextProcessor {
    fn process(&mut self, date: &DateTime<Utc>, text: &str, block_bounding_boxes: &[BoundingBox]);
    fn poll_result(&mut self, date: &DateTime<Utc>) -> Vec<TextItem>;
}

pub struct TextItem {
    pub date: DateTime<Utc>,
    pub text: String,
    pub confidence: f32,
}

struct InputTextItem {
    pub date: DateTime<Utc>,
    pub text: String,
    pub confidence: f32,
    pub previous_similarity: Option<f64>, // [0.0, 1.0]
}

/// Processes text recognition results for region focused on a line of text
/// that does not change position.
pub struct FixedLineProcessor {
    region: Region,
    input_buffer: Vec<InputTextItem>,
    output_buffer: VecDeque<TextItem>,
    similarity_calculator: JaroWinkler,
}

impl FixedLineProcessor {
    pub fn new(region: Region) -> Self {
        Self {
            region,
            input_buffer: Vec::new(),
            output_buffer: VecDeque::new(),
            similarity_calculator: JaroWinkler::new(),
        }
    }

    fn flush_input_to_output_buffer(&mut self) {
        if self.input_buffer.is_empty() {
            return;
        }

        let mut best_confidence = 0.0;
        let mut best_index = 0;

        for (index, item) in self.input_buffer.iter().enumerate() {
            if item.confidence > best_confidence {
                best_confidence = item.confidence;
                best_index = index;
            }
        }

        let best_item = &self.input_buffer[best_index];

        self.output_buffer.push_back(TextItem {
            date: best_item.date,
            text: best_item.text.clone(),
            confidence: best_item.confidence,
        });

        self.input_buffer.clear();
    }
}

impl TextProcessor for FixedLineProcessor {
    fn process(&mut self, date: &DateTime<Utc>, text: &str, block_bounding_boxes: &[BoundingBox]) {
        if is_text_block_confidence_ok(0.6, block_bounding_boxes)
            && is_text_block_top_left(&self.region, block_bounding_boxes)
        {
            let mut previous_similarity = None;

            if let Some(item) = self.input_buffer.first() {
                let similarity = self.similarity_calculator.similarity(&item.text, text);

                previous_similarity = Some(similarity);

                if similarity < 0.8 {
                    self.flush_input_to_output_buffer();
                }
            }

            self.input_buffer.push(InputTextItem {
                text: text.to_owned(),
                date: date.to_owned(),
                confidence: block_bounding_boxes.first().unwrap().confidence,
                previous_similarity,
            });
        }
    }

    fn poll_result(&mut self, date: &DateTime<Utc>) -> Vec<TextItem> {
        if let Some(item) = self.input_buffer.last() {
            if date.signed_duration_since(item.date) > chrono::Duration::seconds(5) {
                self.flush_input_to_output_buffer();
            }
        }

        let mut results = Vec::new();

        while let Some(item) = self.output_buffer.pop_front() {
            results.push(item);
        }

        results
    }
}

/// Processes text recognition results for a region focused on a fixed-size
/// dialog box in which text is revealed glyph-by-glyph and lines may shift up
/// (scroll) to reveal subsequent lines.
pub struct DialogScrollProcessor {
    region: Region,
}

impl DialogScrollProcessor {
    pub fn new(region: Region) -> Self {
        Self { region }
    }
}

impl TextProcessor for DialogScrollProcessor {
    fn process(&mut self, date: &DateTime<Utc>, text: &str, block_bounding_boxes: &[BoundingBox]) {
        // TODO!
    }

    fn poll_result(&mut self, date: &DateTime<Utc>) -> Vec<TextItem> {
        // TODO!
        Vec::new()
    }
}

fn is_text_block_top_left(region: &Region, block_bounding_boxes: &[BoundingBox]) -> bool {
    if let Some(bounding_box) = block_bounding_boxes.first() {
        let ratio_x = (region.x as f32 - bounding_box.x1 as f32) / region.width as f32;
        let ratio_y = (region.y as f32 - bounding_box.y1 as f32) / region.height as f32;

        ratio_x <= 0.2 && ratio_y <= 0.2
    } else {
        false
    }
}

fn is_text_block_confidence_ok(threshold: f32, block_bounding_boxes: &[BoundingBox]) -> bool {
    if let Some(bounding_box) = block_bounding_boxes.first() {
        bounding_box.confidence >= threshold
    } else {
        false
    }
}
