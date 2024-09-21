use std::collections::HashMap;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use socket2::Socket;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use tracing::{debug, error, info};

use crate::{comm, crypt, model};

const DEFAULT_ROOM_TTL: Duration = Duration::from_secs(3 * 3600); // 3 hour
const DEFAULT_ROOM_CLEANUP_INTERVAL: Duration = Duration::from_secs(600); // 10 min

const PING_ROOM: &str = "pinglkasjdlfjsaldjf";
const WEAK_KEY: &[u8] = &[1, 2, 3];

#[allow(non_camel_case_types)]
type roomMap = Arc<RwLock<HashMap<String, roomInfo>>>;

#[allow(non_camel_case_types)]
pub struct roomInfo {
    full: bool,
    opened: Instant,
    first: Option<comm::Comm>,
    second: Option<comm::Comm>,
}

#[allow(non_camel_case_types)]
pub struct server {
    host: String,
    port: String,
    password: String,
    banner: String,
    rooms: roomMap,
    room_cleanup_interval: Duration,
    room_ttl: Duration,
}

// newDefaultServer initializes a new server, with some default configuration options
fn new_default_server(
    host: &str,
    port: String,
    password: String,
    banner: String,
) -> server {
    server {
        host: host.into(),
        port,
        password,
        banner,
        rooms: Arc::new(RwLock::new(HashMap::new())),
        room_ttl: DEFAULT_ROOM_TTL,
        room_cleanup_interval: DEFAULT_ROOM_CLEANUP_INTERVAL,
    }
}

// Run starts a tcp listener, run async
pub fn run(
    host: &str,
    port: String,
    password: String,
    banner: String,
) -> anyhow::Result<()> {
    let s = new_default_server(host, port, password, banner);
    s.start()
}

impl server {
    fn start(&self) -> anyhow::Result<()> {
        debug!(target: "starting with password", password = self.password);

        let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);
        std::thread::spawn({
            let rooms = self.rooms.clone();
            let ttl = self.room_ttl;
            let interval = self.room_cleanup_interval;
            move || {
                delete_old_rooms(rooms, stop_rx, interval, ttl);
            }
        });

        let rst = self.run();
        if let Err(ref e) = rst {
            error!(error = ?e);
        }

        debug!("stop room cleanup fired");
        let _ = stop_tx.send(());

        rst
    }

    fn run(&self) -> anyhow::Result<()> {
        let mut addr = format!("{}:{}", self.host, self.port);
        addr.to_socket_addrs()?;
        addr = addr.replacen("127.0.0.1", "0.0.0.0", 1);
        info!("starting TCP server on {}", addr);
        let listener = match TcpListener::bind(&addr) {
            Err(e) => {
                anyhow::bail!(format!("error listening on {}: {:?}", addr, e))
            },
            Ok(x) => x,
        };

        // spawn a new goroutine whenever a client connects
        std::thread::scope(|s| {
            loop {
                let (stream, addr) = match listener.accept() {
                    Err(e) => {
                        anyhow::bail!(format!("problem accepting connection: {:?}", e))
                    },
                    Ok(x) => x,
                };
                debug!("client {:?} connected", addr);
                let port = self.port.clone();
                let password = self.password.clone();
                let banner = self.banner.clone();
                let rooms = self.rooms.clone();
                let socket = Socket::from(stream);

                s.spawn(move || {
                    let c = comm::new(socket, addr);
                    let room_key =
                        match client_communication(&port, &password, &banner, &rooms, c, &addr) {
                            Err(e) => {
                                debug!("relay-{}: {:?}", addr, e);
                                return;
                            },
                            Ok(x) => x,
                        };
                    debug!(room_key);
                    if room_key == PING_ROOM {
                        debug!("got ping");
                        return;
                    }

                    loop {
                        // check connection
                        debug!(target: "checking connection", room = room_key);
                        let mut delete_it = false;

                        let mut lock = rooms.write();
                        let room: &mut roomInfo = match lock.get_mut(&room_key) {
                            None => {
                                debug!("room is gone");
                                return;
                            },
                            Some(x) => x,
                        };

                        if room.first.is_some() && room.second.is_some() {
                            debug!("rooms ready");
                            drop(lock);
                            break;
                        } else {
                            if let Some(ref mut comm) = room.first {
                                if let Err(e) = comm.send(&[1u8]) {
                                    debug!(?e);
                                    delete_it = true;
                                }
                            }
                        }
                        drop(lock);
                        if delete_it {
                            debug!(target: "deleting room", room = room_key);
                            let mut lock = rooms.write();
                            lock.remove(&room_key);
                            break;
                        }

                        std::thread::sleep(Duration::from_secs(1));
                    }
                });
            }

            Ok::<_, anyhow::Error>(())
        })
    }
}

fn client_communication(
    _port: &str,
    password: &str,
    banner: &str,
    rooms: &roomMap,
    mut c: comm::Comm,
    addr: &SocketAddr,
) -> anyhow::Result<String> {
    // establish secure password with PAKE for communication with relay
    let (b, bbytes) =
        Spake2::<Ed25519Group>::start_symmetric(&Password::new(WEAK_KEY), &Identity::new(b"siec"));
    let abytes = c.receive()?;
    if abytes == b"ping" {
        debug!("sending back pong");
        return Ok(PING_ROOM.into());
    }
    let strong_key = match b.finish(&abytes) {
        Err(e) => {
            anyhow::bail!("{:?}", e)
        },
        Ok(x) => x,
    };
    c.send(&bbytes)?;
    // receive salt
    let salt = c.receive()?;
    let (strong_encryption, _) = crypt::new(&strong_key, &salt)?;
    debug!("waiting for password");
    let password_bytes_enc = c.receive()?;
    let password_bytes = crypt::decrypt(&password_bytes_enc, &strong_encryption)?;
    let passwd = String::from_utf8(password_bytes)?;
    if passwd != password {
        let enc = crypt::encrypt(b"bad password", &strong_encryption)?;
        if let Err(e) = c.send(&enc) {
            anyhow::bail!("send error: {:?}", e)
        }
        anyhow::bail!("bad password")
    }

    // send ok to tell client they are connected
    let baner = if banner.is_empty() { "ok" } else { banner };
    debug!(target: "sending", banner = ?baner);
    let msg = format!("{}|||{}", baner, addr);
    let bsend = crypt::encrypt(msg.as_bytes(), &strong_encryption)?;
    c.send(&bsend)?;
    // wait for client to tell me which room they want
    debug!("waiting for answer");
    let enc = c.receive()?;
    let room_bytes = crypt::decrypt(&enc, &strong_encryption)?;
    let room_key = String::from_utf8(room_bytes)?;

    // create the room if it is new
    let mut lock = rooms.write();
    match lock.get_mut(&room_key) {
        None => {
            let bsend = crypt::encrypt(b"ok", &strong_encryption)?;
            c.send(&bsend)?;
            lock.insert(
                room_key.clone(),
                roomInfo {
                    first: Some(c),
                    second: None,
                    full: false,
                    opened: Instant::now(),
                },
            );
            debug!("room {} has 1", room_key);
            Ok(room_key)
        },
        Some(room) => {
            if room.full {
                let bsend = crypt::encrypt(b"room full", &strong_encryption)?;
                if let Err(e) = c.send(&bsend) {
                    error!(target: "comm_send", error = ?e);
                    return Err(e);
                }

                return Ok(room_key);
            }

            debug!("room {} has 2", room_key);
            room.full = true;

            let first: &mut comm::Comm = room
                .first
                .as_mut()
                .ok_or(anyhow::anyhow!("room: {} first should not be nil", room_key))?;
            // second connection is the sender, time to staple connections
            // tell the sender everything is ready
            let bsend = crypt::encrypt(b"ok", &strong_encryption)?;
            if let Err(_e) = c.send(&bsend) {
                lock.remove(&room_key);
                return Ok(room_key);
            }

            // start piping
            debug!("starting pipes");
            let _ = pipe(first.connection(), c.connection())?;
            debug!("done piping");
            room.second = Some(c);
            lock.remove(&room_key);
            Ok(room_key)
        },
    }
}

fn pipe(
    a: &mut Socket,
    b: &mut Socket,
) -> anyhow::Result<()> {
    let (txa, rxa) = crossbeam_channel::bounded::<Vec<u8>>(2);
    let (txb, rxb) = crossbeam_channel::bounded::<Vec<u8>>(2);
    let writera = a.try_clone()?;
    let writerb = b.try_clone()?;

    std::thread::scope(|s| {
        s.spawn(|| {
            while let Ok(u8s) = rxa.recv() {
                if let Err(e) = writerb.send(&u8s) {
                    error!(target: "write error on channel 2", error = ?e);
                }
            }
        });
        s.spawn(|| {
            loop {
                let mut buf = Vec::with_capacity(model::TCP_BUFFER_SIZE);
                match a.read(&mut buf) {
                    Ok(n) => {
                        buf.drain(n..);
                        let _ = txa.send(buf.clone());
                    },
                    Err(e) => {
                        debug!(?e);
                        drop(txa);
                        break;
                    },
                }
            }

            debug!("exiting");
        });

        s.spawn(|| {
            while let Ok(u8s) = rxb.recv() {
                if let Err(e) = writera.send(&u8s) {
                    error!(target: "write error on channel 1", error = ?e);
                }
            }
        });
        s.spawn(|| {
            loop {
                let mut buf = Vec::with_capacity(model::TCP_BUFFER_SIZE);
                match b.read(&mut buf) {
                    Ok(n) => {
                        buf.drain(n..);
                        let _ = txb.send(buf.clone());
                    },
                    Err(e) => {
                        debug!(?e);
                        drop(txb);
                        break;
                    },
                }
            }

            debug!("exiting");
        });
    });

    Ok(())
}

fn delete_old_rooms(
    rooms: roomMap,
    mut stop_rx: crossbeam_channel::Receiver<()>,
    room_cleanup_interval: Duration,
    room_ttl: Duration,
) {
    loop {
        std::thread::sleep(room_cleanup_interval);
        match stop_rx.try_recv() {
            Ok(_) => {
                return;
            },
            Err(e) => {
                if e.is_disconnected() {
                    return;
                }
            },
        }

        let mut rooms_to_delete = vec![];
        {
            let lock = rooms.read();
            for (key, room) in lock.iter() {
                if room.opened.elapsed() > room_ttl {
                    rooms_to_delete.push(key.to_string());
                }
            }
        }
        {
            let mut lock = rooms.write();
            for room in rooms_to_delete {
                lock.remove(&room);
                debug!(target: "room cleaned up", room = room);
            }
        }
    }
}
