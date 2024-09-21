use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use anyhow::Context;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use parking_lot::RwLock;
use socket2::Socket;
use tracing::{debug, warn};

const MAGIC_BYTES: &[u8] = b"croc";

lazy_static::lazy_static! {
    pub static ref SOCKS5_PROXY: RwLock<String> = RwLock::new(String::from(""));
    pub static ref HTTP_PROXY: RwLock<String> = RwLock::new(String::from(""));
}

// Comm is some basic TCP communication
#[derive(Debug)]
pub struct Comm {
    socket: Socket,
    addr: SocketAddr,
}

impl Comm {
    pub fn connection(&mut self) -> &mut Socket {
        &mut self.socket
    }
}

pub fn new(
    stream: Socket,
    addr: SocketAddr,
) -> Comm {
    Comm { socket: stream, addr }
}

// Send a message
impl Comm {
    pub fn send(
        &mut self,
        message: &[u8],
    ) -> anyhow::Result<()> {
        let _ = self.write(message)?;
        Ok(())
    }

    pub fn write(
        &mut self,
        message: &[u8],
    ) -> anyhow::Result<usize> {
        let mut buf = Vec::new();
        for it in MAGIC_BYTES {
            buf.push(*it);
        }
        WriteBytesExt::write_u32::<LittleEndian>(&mut buf, message.len() as u32)?;
        for it in message {
            buf.push(*it);
        }
        // let n = match self.stream.write_all(&buf).await {
        let n = match self.socket.send(&buf) {
            Err(e) => {
                anyhow::bail!("connection.Write failed: {:?}", e)
            },
            Ok(x) => x,
        };
        if n != buf.len() {
            anyhow::bail!(format!("wanted to write {} but wrote {}", buf.len(), n))
        }

        Ok(n)
    }

    pub fn receive(&mut self) -> anyhow::Result<Vec<u8>> {
        let (b, _) = self.read()?;
        Ok(b)
    }

    pub fn read(&mut self) -> anyhow::Result<(Vec<u8>, usize)> {
        // long read deadline in case waiting for file
        let mut header = vec![0; 4];
        if let Err(e) = self.socket.set_read_timeout(Some(Duration::from_secs(3 * 3600))) {
            warn!(target: "setting read deadline", error = ?e);
        }
        self.socket.read(&mut header).context("initial read error")?;
        if header != MAGIC_BYTES {
            anyhow::bail!("initial bytes are not magic: {:?}", header)
        }
        // read until we get 4 bytes for the header
        header = vec![0; 4];
        self.socket.read(&mut header).context("initial read error")?;

        let num_bytes: u32 = LittleEndian::read_u32(&header);
        // shorten the reading deadline in case getting weird data
        if let Err(e) = self.socket.set_read_timeout(Some(Duration::from_secs(10))) {
            warn!(target: "setting read deadline", error = ?e);
        }
        let mut buf = vec![0; num_bytes as usize];
        self.socket.read(&mut buf).context("consecutive read error")?;

        Ok((buf, num_bytes as usize))
    }
}
