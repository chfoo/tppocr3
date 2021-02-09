use std::path::PathBuf;
use std::{ffi::c_void, os::unix::io::RawFd, path::Path};

use anyhow::Context;
use nix::{
    fcntl::{FlockArg, OFlag},
    sys::{
        mman::{MapFlags, ProtFlags},
        stat::Mode,
    },
};

pub struct SharedMemory {
    data_size: usize,
    shared_memory_name: PathBuf,
    shared_memory_fd: RawFd,
    shared_memory: *mut c_void,
    unlink_on_drop: bool,
}

impl SharedMemory {
    pub fn open_or_create(id: u32, data_size: usize) -> anyhow::Result<Self> {
        Self::open_(id, data_size, true)
    }

    pub fn open(id: u32, data_size: usize) -> anyhow::Result<Self> {
        Self::open_(id, data_size, false)
    }

    fn open_(id: u32, data_size: usize, create: bool) -> anyhow::Result<Self> {
        // File is mounted to /dev/shm/
        let shared_memory_name = PathBuf::from(format!("/tppocr_{}", id));
        let (shared_memory_fd, shared_memory) =
            Self::open_shared_memory(data_size, &shared_memory_name, create)?;

        Ok(Self {
            data_size,
            shared_memory_name,
            shared_memory_fd,
            shared_memory,
            unlink_on_drop: false,
        })
    }

    pub fn data(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.shared_memory as *const u8, self.data_size) }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.shared_memory as *mut u8, self.data_size) }
    }

    pub fn data_32(&self) -> &[u32] {
        unsafe { std::slice::from_raw_parts(self.shared_memory as *const u32, self.data_size / 4) }
    }

    pub fn data_32_mut(&mut self) -> &mut [u32] {
        unsafe {
            std::slice::from_raw_parts_mut(self.shared_memory as *mut u32, self.data_size / 4)
        }
    }

    pub fn data_raw(&mut self) -> *mut c_void {
        self.shared_memory
    }

    pub fn unlink_on_drop(&self) -> bool {
        self.unlink_on_drop
    }

    pub fn set_unlink_on_drop(&mut self, value: bool) {
        self.unlink_on_drop = value;
    }

    pub fn lock(&self) -> anyhow::Result<()> {
        nix::fcntl::flock(self.shared_memory_fd, FlockArg::LockExclusive)?;

        Ok(())
    }

    pub fn unlock(&self) -> anyhow::Result<()> {
        nix::fcntl::flock(self.shared_memory_fd, FlockArg::Unlock)?;

        Ok(())
    }

    fn open_shared_memory(
        data_size: usize,
        shared_memory_name: &Path,
        create: bool,
    ) -> anyhow::Result<(RawFd, *mut c_void)> {
        let shm_flags = if create {
            OFlag::O_RDWR | OFlag::O_CREAT
        } else {
            OFlag::O_RDWR
        };
        let mode_flags = Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP;
        let fd = nix::sys::mman::shm_open(shared_memory_name, shm_flags, mode_flags)
            .with_context(|| format!("Failed to open shared memory {:?}", shared_memory_name))?;

        nix::unistd::ftruncate(fd, data_size as i64)?;

        let pointer = unsafe {
            nix::sys::mman::mmap(
                std::ptr::null_mut(),
                data_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd,
                0,
            )
            .context("Failed to open memory map")?
        };

        Ok((fd, pointer))
    }

    pub fn unlink(self) -> anyhow::Result<()> {
        nix::sys::mman::shm_unlink(&self.shared_memory_name)?;

        Ok(())
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        if self.unlink_on_drop {
            nix::sys::mman::shm_unlink(&self.shared_memory_name).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write() -> anyhow::Result<()> {
        let mut shared_memory = SharedMemory::open_or_create(123, 100)?;

        shared_memory.lock()?;
        shared_memory.data_mut()[4] = 2;
        assert_eq!(shared_memory.data()[4], 2);
        shared_memory.unlock()?;
        shared_memory.unlink()?;

        Ok(())
    }
}
