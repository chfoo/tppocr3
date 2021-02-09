use std::{
    os::unix::net::UnixDatagram,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;

pub struct MessageServer {
    path: PathBuf,
    socket: UnixDatagram,
}

impl MessageServer {
    pub fn open(id: u32) -> anyhow::Result<Self> {
        let path = PathBuf::from(format!("/tmp/tppocr_{}.socket", id));

        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        let socket = UnixDatagram::bind(&path)?;

        Ok(Self { path, socket })
    }

    pub fn set_nonblocking(&self, value: bool) -> anyhow::Result<()> {
        Ok(self.socket.set_nonblocking(value)?)
    }

    pub fn set_timeout(&self, value: Option<Duration>) -> anyhow::Result<()> {
        self.socket.set_read_timeout(value)?;
        self.socket.set_write_timeout(value)?;
        Ok(())
    }

    pub fn send(&self, buffer: &[u8], client: &Path) -> anyhow::Result<usize> {
        Ok(self.socket.send_to(buffer, client)?)
    }

    pub fn receive(&self, buffer: &mut [u8]) -> anyhow::Result<(usize, PathBuf)> {
        let (size, address) = self.socket.recv_from(buffer)?;

        if let Some(path) = address.as_pathname() {
            Ok((size, path.to_path_buf()))
        } else {
            Err(anyhow::anyhow!("Client has no named address"))
        }
    }
}

impl Drop for MessageServer {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).unwrap();
    }
}

pub struct MessageClient {
    path: PathBuf,
    socket: UnixDatagram,
}

impl MessageClient {
    pub fn open(id: u32) -> anyhow::Result<Self> {
        let path = PathBuf::from(format!("/tmp/tppocr_client-{}.socket", id));

        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        let server_path = PathBuf::from(format!("/tmp/tppocr_{}.socket", id));
        let socket = UnixDatagram::bind(&path)?;
        socket
            .connect(server_path)
            .with_context(|| format!("Couldn't connect to message server socket {}", id))?;

        Ok(Self { path, socket })
    }

    pub fn set_nonblocking(&self, value: bool) -> anyhow::Result<()> {
        Ok(self.socket.set_nonblocking(value)?)
    }

    pub fn set_timeout(&self, value: Option<Duration>) -> anyhow::Result<()> {
        self.socket.set_read_timeout(value)?;
        self.socket.set_write_timeout(value)?;
        Ok(())
    }

    pub fn send(&self, buffer: &[u8]) -> anyhow::Result<usize> {
        Ok(self.socket.send(buffer)?)
    }

    pub fn receive(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        Ok(self.socket.recv(buffer)?)
    }
}

impl Drop for MessageClient {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_client() -> anyhow::Result<()> {
        let server = MessageServer::open(123)?;
        let client = MessageClient::open(123)?;

        server.set_timeout(Some(Duration::from_secs(5)))?;
        client.set_timeout(Some(Duration::from_secs(5)))?;

        let mut client_buffer: [u8; 10] = [0; 10];
        let mut server_buffer: [u8; 10] = [0; 10];

        client_buffer[0] = 123;
        client.send(&client_buffer)?;

        let (message_size, client_name) = server.receive(&mut server_buffer)?;

        assert_eq!(message_size, 10);
        assert_eq!(server_buffer[0], 123);

        server_buffer[0] = 234;
        server.send(&server_buffer, &client_name)?;

        let message_size = client.receive(&mut client_buffer)?;
        assert_eq!(message_size, 10);
        assert_eq!(client_buffer[0], 234);

        Ok(())
    }
}
