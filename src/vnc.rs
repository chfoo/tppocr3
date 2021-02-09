use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{bail, Context};
use slog_scope::info;

use crate::{bindings::vnc, shared_memory::SharedMemory};

const BYTES_PER_PIXEL: u32 = 4;

pub struct VncServer {
    port: u16,
    width: u32,
    height: u32,
    shared_memory: SharedMemory,
    frame_buffer: Vec<u32>,
}

impl VncServer {
    pub fn new(port: u16, width: u32, height: u32) -> anyhow::Result<Self> {
        let pixel_count = (width * height) as usize;
        let data_size = (width * height * BYTES_PER_PIXEL) as usize;

        let shared_memory = SharedMemory::open_or_create(port as u32, data_size)?;
        // Coordinating process should unlink the shared memory

        let mut frame_buffer = Vec::<u32>::new();
        frame_buffer.resize(pixel_count, 0);

        Ok(Self {
            port,
            width,
            height,
            shared_memory,
            frame_buffer,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let screen_info = self.create_screen()?;
        self.set_up_screen(screen_info);

        info!("loop start");

        let terminate_flag = Arc::new(AtomicBool::new(false));
        for sig in signal_hook::consts::TERM_SIGNALS {
            signal_hook::flag::register(*sig, Arc::clone(&terminate_flag)).unwrap();
        }

        while unsafe { vnc::rfbIsActive(screen_info) != 0 } {
            self.shared_memory.lock()?;
            // let rect = self.get_change_rect();
            self.frame_buffer
                .copy_from_slice(self.shared_memory.data_32());
            self.shared_memory.unlock()?;

            // if let Some((x1, y1, x2, y2)) = rect {
            //     unsafe {
            //         vnc::rfbMarkRectAsModified(screen_info, x1, y1, x2, y2);
            //     }
            // }
            unsafe {
                vnc::rfbMarkRectAsModified(
                    screen_info,
                    0,
                    0,
                    self.width as i32,
                    self.height as i32,
                );
                vnc::rfbProcessEvents(screen_info, (*screen_info).deferUpdateTime as i64 * 1000);
            }

            if terminate_flag.load(Ordering::Relaxed) {
                info!("server shutdown");
                unsafe {
                    vnc::rfbShutdownServer(screen_info, 1);
                }
            }
        }

        info!("loop stop");

        Ok(())
    }

    pub fn create_screen(&self) -> anyhow::Result<vnc::rfbScreenInfoPtr> {
        let mut argc = 0;
        let screen_info = unsafe {
            vnc::rfbGetScreen(
                &mut argc,
                std::ptr::null_mut(),
                self.width as i32,
                self.height as i32,
                8,
                3,
                BYTES_PER_PIXEL as i32,
            )
        };

        if screen_info.is_null() {
            bail!("get libvnc screen error");
        } else {
            Ok(screen_info)
        }
    }

    fn set_up_screen(&mut self, screen_info: vnc::rfbScreenInfoPtr) {
        unsafe {
            (*screen_info).frameBuffer = self.frame_buffer.as_mut_ptr() as *mut i8;
            (*screen_info).alwaysShared = 1;
            (*screen_info).deferUpdateTime = 200;
            (*screen_info).autoPort = 0;
            (*screen_info).port = self.port as i32;
            (*screen_info).ipv6port = 0; // disable IPv6

            // Bind to a loopback interface, not public, for security good practices.
            // A HTTP reverse proxy server can forward a Novnc websocket.
            (*screen_info).listenInterface = 0x7f00_0001u32.to_be();

            // Have to call rfbInitServer() with the expanded macro
            vnc::rfbInitServerWithPthreadsAndZRLE(screen_info);
        }
    }

    fn _get_change_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let pixel_count = (self.width * self.height) as usize;

        let mut min_x: i32 = i32::MAX;
        let mut max_x: i32 = i32::MIN;
        let mut min_y: i32 = i32::MAX;
        let mut max_y: i32 = i32::MIN;
        let mut has_changes = false;

        for pixel_index in 0..pixel_count {
            let new_pixel = self.shared_memory.data_32()[pixel_index];
            let old_pixel = self.frame_buffer[pixel_index];
            let x = pixel_index as u32 % self.width;
            let y = pixel_index as u32 / self.width;

            if new_pixel != old_pixel {
                min_x = min_x.min(x as i32);
                max_x = max_x.max(x as i32);
                min_y = min_y.min(y as i32);
                max_y = max_y.max(y as i32);
                has_changes = true;
            }
        }

        if has_changes {
            Some((min_x, min_y, max_x + 1, max_y + 1))
        } else {
            None
        }
    }
}

pub struct VncClient {
    width: u32,
    height: u32,
    shared_memory: SharedMemory,
}

impl VncClient {
    pub fn new(port: u16, width: u32, height: u32) -> anyhow::Result<Self> {
        let data_size = (width * height * BYTES_PER_PIXEL) as usize;

        let shared_memory = SharedMemory::open_or_create(port as u32, data_size)?;

        Ok(Self {
            width,
            height,
            shared_memory,
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

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.shared_memory.data_mut()
    }

    pub fn data_u32(&self) -> &[u32] {
        self.shared_memory.data_32()
    }

    pub fn data_u32_mut(&mut self) -> &mut [u32] {
        self.shared_memory.data_32_mut()
    }

    pub fn lock(&self) -> anyhow::Result<()> {
        self.shared_memory
            .lock()
            .context("Failed to lock shared memory")
    }

    pub fn unlock(&self) -> anyhow::Result<()> {
        self.shared_memory
            .unlock()
            .context("Failed to unlock shared memory")
    }
}
