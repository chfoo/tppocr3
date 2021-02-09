use std::{
    convert::TryInto,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use chrono::Utc;
use image::{Bgra, ImageBuffer};
use raqote::{Color, DrawOptions, DrawTarget, Image, PathBuilder, Point, Source, StrokeStyle};
use slog_scope::info;

use crate::{canvas::TextDrawer, config::{ProcessorConfig, ProcessorStrategy, Region}, frame::FrameReader, text_processor::{DialogScrollProcessor, FixedLineProcessor, TextItem, TextProcessor}, text_recognizer::TextRecognizer, vnc::VncClient};

pub struct Processor {
    frame_reader: FrameReader,
    vnc_client: VncClient,
    text_recognizer: TextRecognizer,
    region_processors: Vec<RegionProcessor>,
    config: ProcessorConfig,
    canvas: DrawTarget,
    text_drawer: TextDrawer,
    frame_counter: u64,
}

impl Processor {
    pub fn new(
        frame_reader: FrameReader,
        vnc_client: VncClient,
        text_recognizer: TextRecognizer,
        config: ProcessorConfig,
    ) -> Self {
        let canvas = DrawTarget::new(
            vnc_client.width().try_into().unwrap(),
            vnc_client.height().try_into().unwrap(),
        );

        let mut region_processors = Vec::new();

        for region in &config.region {
            region_processors.push(RegionProcessor::new(region.clone()));
        }

        Self {
            frame_reader,
            vnc_client,
            text_recognizer,
            region_processors,
            config,
            canvas,
            text_drawer: TextDrawer::new().unwrap(),
            frame_counter: 0,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let terminate_flag = Arc::new(AtomicBool::new(false));
        for sig in signal_hook::consts::TERM_SIGNALS {
            signal_hook::flag::register(*sig, Arc::clone(&terminate_flag)).unwrap();
        }

        info!("starting");

        while !terminate_flag.load(Ordering::Relaxed) {
            self.process_frame()?;
            self.draw_date();

            self.vnc_client.lock()?;
            self.vnc_client
                .data_u32_mut()
                .copy_from_slice(self.canvas.get_data());
            self.vnc_client.unlock()?;

            self.frame_counter += 1;
        }

        info!("exiting");

        Ok(())
    }

    fn process_frame(&mut self) -> anyhow::Result<()> {
        self.frame_reader.read()?;

        self.text_recognizer.set_image(
            self.frame_reader.data_u32(),
            self.frame_reader.width(),
            self.frame_reader.height(),
        );

        self.clear_canvas();

        let mut draw_offset_y = 0;

        for region_processor in &mut self.region_processors {
            region_processor.process(
                &self.text_recognizer,
                &self.frame_reader,
                &mut self.canvas,
                draw_offset_y,
            )?;

            draw_offset_y += region_processor.region().height as i32 + 48;

            for text_item in region_processor.get_text() {
                dbg!(text_item.date, text_item.text);
            }
        }

        Ok(())
    }

    fn draw_date(&mut self) {
        let color = Color::new(255, 255, 255, 255);
        self.text_drawer.set_color(color);
        self.text_drawer
            .set_position(Point::new(0.0, self.vnc_client.height() as f32));

        self.text_drawer.draw(
            &mut self.canvas,
            &format!("Date={} FrameCounter={}", Utc::now(), self.frame_counter),
        );
    }

    fn clear_canvas(&mut self) {
        let source = Source::from(Color::new(255, 0, 0, 0));
        let options = DrawOptions::default();
        self.canvas.fill_rect(
            0.0,
            0.0,
            self.canvas.width() as f32,
            self.canvas.height() as f32,
            &source,
            &options,
        );
    }
}

struct RegionProcessor {
    region: Region,
    text_drawer: TextDrawer,
    text_processor: Box<dyn TextProcessor>,
}

impl RegionProcessor {
    pub fn new(region: Region) -> Self {
        Self {
            region: region.clone(),
            text_drawer: TextDrawer::new().unwrap(),
            text_processor: Self::get_text_processor(region),
        }
    }

    fn get_text_processor(region: Region) -> Box<dyn TextProcessor> {
        match region.processor {
            ProcessorStrategy::FixedLine => Box::new(FixedLineProcessor::new(region)),
            ProcessorStrategy::DialogScroll => Box::new(DialogScrollProcessor::new(region)),
        }
    }

    pub fn region(&self) -> &Region {
        &self.region
    }

    pub fn process(
        &mut self,
        text_recognizer: &TextRecognizer,
        frame_reader: &FrameReader,
        canvas: &mut DrawTarget,
        draw_offset_y: i32,
    ) -> anyhow::Result<()> {
        text_recognizer.set_rectangle(
            self.region.x,
            self.region.y,
            self.region.width,
            self.region.height,
        );
        text_recognizer.recognize()?;

        self.draw_image(frame_reader, canvas, draw_offset_y);
        self.draw_region_bounding_boxes(text_recognizer, canvas, draw_offset_y);

        let text = text_recognizer.get_text();
        let bounding_boxes = text_recognizer.get_block_boxes();
        let date = Utc::now();

        self.text_processor.process(&date, &text, &bounding_boxes);

        self.draw_text(&text, canvas, draw_offset_y);

        Ok(())
    }

    fn draw_image(&self, frame_reader: &FrameReader, canvas: &mut DrawTarget, draw_offset_y: i32) {
        let image = ImageBuffer::<Bgra<u8>, _>::from_raw(
            frame_reader.width(),
            frame_reader.height(),
            frame_reader.data().to_vec(),
        )
        .unwrap();
        let subimage = image::imageops::crop_imm(
            &image,
            self.region.x,
            self.region.y,
            self.region.width,
            self.region.height,
        );
        let subimage = subimage.to_image();
        let canvas_image = Image {
            width: self.region.width as i32,
            height: self.region.height as i32,
            data: unsafe {
                std::slice::from_raw_parts(
                    subimage.as_raw().as_ptr() as *const u32,
                    subimage.as_raw().len() / 4,
                )
            },
        };

        let options = DrawOptions::new();
        canvas.draw_image_at(0.0, draw_offset_y as f32, &canvas_image, &options);
    }

    fn draw_region_bounding_boxes(
        &mut self,
        text_recognizer: &TextRecognizer,
        canvas: &mut DrawTarget,
        draw_offset_y: i32,
    ) {
        for bounding_box in text_recognizer.get_word_boxes() {
            let mut path = PathBuilder::new();
            path.rect(
                (bounding_box.x1 - self.region.x as i32) as f32,
                (bounding_box.y1 - self.region.y as i32 + draw_offset_y) as f32,
                (bounding_box.x2 - bounding_box.x1) as f32,
                (bounding_box.y2 - bounding_box.y1) as f32,
            );
            let path = path.finish();
            let source = Source::from(Color::new(255, 0, 255, 0));
            let style = StrokeStyle::default();
            let options = DrawOptions::new();

            canvas.stroke(&path, &source, &style, &options);

            self.text_drawer.set_color(Color::new(255, 0, 255, 0));
            self.text_drawer.set_position(Point::new(
                (bounding_box.x1 - self.region.x as i32) as f32,
                (bounding_box.y1 - self.region.y as i32 + draw_offset_y) as f32,
            ));
            self.text_drawer
                .draw(canvas, &format!("{:.3}", bounding_box.confidence));
        }
    }

    fn draw_text(&mut self, text: &str, canvas: &mut DrawTarget, draw_offset_y: i32) {
        self.text_drawer.set_color(Color::new(255, 255, 0, 255));
        self.text_drawer.set_position(Point::new(
            0.0,
            self.region.height as f32 + draw_offset_y as f32 + 16.0,
        ));
        self.text_drawer.draw(canvas, text);
    }

    pub fn get_text(&mut self) -> Vec<TextItem> {
        let date = Utc::now();
        self.text_processor.poll_result(&date)
    }
}
