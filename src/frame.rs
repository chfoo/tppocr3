use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Context;
use ffmpeg_next::{decoder::Video, format::Pixel, frame, media::Type, software::scaling};
use slog_scope::{info, warn};

use crate::{
    message_socket::{MessageClient, MessageServer},
    shared_memory::SharedMemory,
};

const BYTES_PER_PIXEL: u32 = 4;

pub struct FrameDumper {
    url: String,
    output_width: u32,
    output_height: u32,
    shared_memory: SharedMemory,
    message_server: MessageServer,
    previous_presentation_time: f64,
    decoded_frame: frame::video::Video,
    rgb_frame: frame::video::Video,
    infinite_loop: bool,
    skip_sleep: bool,
}

impl FrameDumper {
    pub fn new(
        url: String,
        output_port: u16,
        output_width: u32,
        output_height: u32,
    ) -> anyhow::Result<Self> {
        let data_size = (output_width * output_height * BYTES_PER_PIXEL) as usize;

        let shared_memory = SharedMemory::open_or_create(output_port as u32, data_size)?;
        // Coordinating process should unlink the shared memory

        let message_server = MessageServer::open(output_port as u32)?;
        message_server.set_nonblocking(true)?;

        Ok(Self {
            url,
            output_width,
            output_height,
            shared_memory,
            message_server,
            previous_presentation_time: 0.0,
            decoded_frame: frame::video::Video::empty(),
            rgb_frame: frame::video::Video::empty(),
            infinite_loop: false,
            skip_sleep: false,
        })
    }

    pub fn infinite_loop(&self) -> bool {
        self.infinite_loop
    }

    pub fn set_infinite_loop(&mut self, value: bool) {
        self.infinite_loop = value;
    }

    pub fn skip_sleep(&self) -> bool {
        self.skip_sleep
    }

    pub fn set_skip_sleep(&mut self, value: bool) {
        self.skip_sleep = value;
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut input = ffmpeg_next::format::input(&PathBuf::from(&self.url))?;
        let video_stream = input
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg_next::Error::StreamNotFound)?;
        let video_stream_index = video_stream.index();
        let time_base = video_stream.time_base().numerator() as f64
            / video_stream.time_base().denominator() as f64;

        let mut decoder = video_stream.codec().decoder().video()?;
        let mut scaler = self.make_scaler(&decoder)?;

        info!("loop start");

        let terminate_flag = Arc::new(AtomicBool::new(false));
        for sig in signal_hook::consts::TERM_SIGNALS {
            signal_hook::flag::register(*sig, Arc::clone(&terminate_flag)).unwrap();
        }

        loop {
            for (stream, packet) in input.packets() {
                if stream.index() == video_stream_index {
                    decoder.send_packet(&packet)?;

                    while let Ok(true) = Self::process_receive_frame_result(
                        decoder.receive_frame(&mut self.decoded_frame),
                    ) {
                        if self.has_frame_format_changed(&scaler) {
                            warn!("frame format changed");
                            scaler = self.make_scaler(&decoder)?;
                        }

                        self.process_frame(&mut scaler, time_base)?;
                    }
                }

                if terminate_flag.load(Ordering::Relaxed) {
                    info!("stopping");
                    self.infinite_loop = false;
                    break;
                }
            }

            if self.infinite_loop {
                input.seek(0, 0..0)?;
                self.previous_presentation_time = 0.0;
            } else {
                break;
            }
        }

        info!("loop stop");

        Ok(())
    }

    fn make_scaler(&self, decoder: &Video) -> anyhow::Result<scaling::context::Context> {
        Ok(scaling::context::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGBA,
            self.output_width,
            self.output_height,
            scaling::Flags::FAST_BILINEAR,
        )?)
    }

    fn process_receive_frame_result(
        result: Result<(), ffmpeg_next::Error>,
    ) -> anyhow::Result<bool> {
        match result {
            Ok(()) => Ok(true),
            Err(error) => match error {
                ffmpeg_next::Error::Eof => Ok(false),
                ffmpeg_next::Error::Other { errno } => match nix::errno::Errno::from_i32(errno) {
                    nix::errno::Errno::EAGAIN => Ok(false),
                    _ => Err(error.into()),
                },
                _ => Err(error.into()),
            },
        }
    }

    fn has_frame_format_changed(&self, scaler: &scaling::context::Context) -> bool {
        // the stream is not guaranteed to have the same format or resolution due to
        // ad injection
        scaler.input().format != self.decoded_frame.format()
            || scaler.input().width != self.decoded_frame.width()
            || scaler.input().height != self.decoded_frame.height()
    }

    fn process_frame(
        &mut self,
        scaler: &mut scaling::context::Context,
        time_base: f64,
    ) -> anyhow::Result<()> {
        let presentation_time = self.decoded_frame.pts().unwrap() as f64 * time_base;
        let mut message_buffer: [u8; 0] = [0; 0];

        if presentation_time - self.previous_presentation_time > 0.1
            || self.previous_presentation_time == 0.0
        {
            let receive_result = self.message_server.receive(&mut message_buffer);

            if receive_result.is_err() {
                return Ok(());
            }

            let (_message_size, client_name) = receive_result.unwrap();

            scaler.run(&self.decoded_frame, &mut self.rgb_frame)?;

            self.shared_memory
                .data_mut()
                .copy_from_slice(self.rgb_frame.data(0));

            self.previous_presentation_time = presentation_time;

            let _ = self.message_server.send(&message_buffer, &client_name);
            // discard error because the client may have disconnected

            if !self.skip_sleep {
                std::thread::sleep(Duration::from_secs_f32(0.1));
            }
        }

        Ok(())
    }
}

pub struct FrameReader {
    width: u32,
    height: u32,
    shared_memory: SharedMemory,
    message_client: MessageClient,
}

impl FrameReader {
    pub fn new(port: u16, width: u32, height: u32) -> anyhow::Result<Self> {
        let data_size = (width * height * BYTES_PER_PIXEL) as usize;

        let shared_memory = SharedMemory::open_or_create(port as u32, data_size)?;

        let message_client = MessageClient::open(port as u32)?;
        message_client.set_timeout(Some(Duration::from_secs(2)))?;

        let mut buffer = Vec::new();
        buffer.resize(data_size, 0);

        Ok(Self {
            width,
            height,
            shared_memory,
            message_client,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        self.shared_memory.data()
    }

    pub fn data_u32(&self) -> &[u32] {
        self.shared_memory.data_32()
    }

    pub fn read(&mut self) -> anyhow::Result<()> {
        let mut message_buffer = [0u8; 0];
        self.message_client.send(&message_buffer)?;
        self.message_client
            .receive(&mut message_buffer)
            .with_context(|| "Disconnected or error sending message to message server")?;

        Ok(())
    }
}
