//! High-performance TCP MPC network
//!
//! Optimized over [`crate::tcp::TcpNetwork`] with:
//! - Write-side buffering via `BufWriter` to coalesce small writes
//! - Vectored I/O (`write_vectored`) to merge length prefix + payload into one syscall
//! - Read-side buffering via `BufReader` to reduce read syscalls
//! - Larger socket buffers (8 MB) for high-bandwidth links
//! - Larger channel capacity (4096) to avoid blocking the reader thread
//! - Per-connection `BufWriter` reduces lock contention pressure

use std::{
    array,
    cmp::Ordering,
    io::{BufReader, BufWriter, IoSlice, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    sync::atomic::AtomicUsize,
    time::{Duration, Instant},
};

use crate::{
    ConnectionStats, DEFAULT_CONNECTION_TIMEOUT, DEFAULT_MAX_FRAME_LENTH, Network, config::Address,
};
use byteorder::{BigEndian, ReadBytesExt as _, WriteBytesExt as _};
use crossbeam_channel::Receiver;
use eyre::ContextCompat;
use intmap::IntMap;
use itertools::Itertools;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use socket2::{Domain, Socket, TcpKeepalive, Type};

// ── Tuning constants ──────────────────────────────────────────
const SOCKET_BUF_SIZE: usize = 8 * 1024 * 1024; // 8 MB kernel socket buffer
const WRITE_BUF_SIZE: usize = 2 * 1024 * 1024; // 2 MB userspace write buffer
const READ_BUF_SIZE: usize = 2 * 1024 * 1024; // 2 MB userspace read buffer
const CHANNEL_CAP: usize = 4096; // crossbeam channel capacity

/// A party in the network.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct NetworkParty {
    /// The id of the party, 0-based indexing.
    pub id: usize,
    /// The DNS name of the party.
    pub dns_name: Address,
}

impl NetworkParty {
    /// Construct a new [`NetworkParty`] type.
    pub fn new(id: usize, address: Address) -> Self {
        Self {
            id,
            dns_name: address,
        }
    }
}

/// The network configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct NetworkConfig {
    /// The list of parties in the network.
    pub parties: Vec<NetworkParty>,
    /// Our own id in the network.
    pub my_id: usize,
    /// The [SocketAddr] we bind to.
    pub bind_addr: SocketAddr,
    /// The connection timeout
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    pub timeout: Option<Duration>,
    /// The max length (in bytes) of a single frame
    #[serde(default)]
    pub max_frame_length: Option<usize>,
}

impl NetworkConfig {
    /// Construct a new [`NetworkConfig`] type.
    pub fn new(
        id: usize,
        bind_addr: SocketAddr,
        parties: Vec<NetworkParty>,
        timeout: Option<Duration>,
        max_frame_length: Option<usize>,
    ) -> Self {
        Self {
            parties,
            my_id: id,
            bind_addr,
            timeout,
            max_frame_length,
        }
    }
}


/// A high-performance MPC network over TCP.
///
/// Compared to [`crate::tcp::TcpNetwork`], this implementation adds
/// userspace buffering on both the write and read paths, uses vectored
/// I/O to merge the 8-byte length prefix with the payload, and sizes
/// kernel socket buffers for 20 Gbps links.
#[derive(Debug)]
#[expect(clippy::complexity)]
pub struct FastTcpNetwork {
    id: usize,
    send: IntMap<usize, (Mutex<BufWriter<TcpStream>>, AtomicUsize)>,
    recv: IntMap<usize, (Receiver<eyre::Result<Vec<u8>>>, AtomicUsize)>,
    timeout: Duration,
    max_frame_length: usize,
}

impl FastTcpNetwork {
    /// Create a single [`FastTcpNetwork`].
    pub fn new(config: NetworkConfig) -> eyre::Result<Self> {
        let [net] = Self::networks::<1>(config)?;
        Ok(net)
    }

    /// Create `N` [`FastTcpNetwork`]s that share the same physical TCP
    /// connections (one per (i, other_id) pair).
    pub fn networks<const N: usize>(config: NetworkConfig) -> eyre::Result<[Self; N]> {
        let id = config.my_id;
        let bind_addr = config.bind_addr;
        let addrs = config
            .parties
            .into_iter()
            .sorted_by_key(|p| p.id)
            .map(|party| party.dns_name)
            .collect::<Vec<_>>();
        let timeout = config.timeout.unwrap_or(DEFAULT_CONNECTION_TIMEOUT);
        let max_frame_length = config.max_frame_length.unwrap_or(DEFAULT_MAX_FRAME_LENTH);

        // ── Listener ──────────────────────────────────────────
        let domain = match bind_addr {
            SocketAddr::V4(_) => Domain::IPV4,
            SocketAddr::V6(_) => Domain::IPV6,
        };
        let socket = Socket::new(domain, Type::STREAM, None)?;
        socket.set_send_buffer_size(SOCKET_BUF_SIZE)?;
        socket.set_recv_buffer_size(SOCKET_BUF_SIZE)?;
        socket.set_reuse_address(true)?;
        if bind_addr.is_ipv6() {
            socket.set_only_v6(false)?;
        }
        socket.set_read_timeout(Some(timeout))?;
        let keepalive = TcpKeepalive::new().with_interval(Duration::from_secs(1));
        socket.set_tcp_keepalive(&keepalive)?;
        socket.bind(&bind_addr.into())?;
        socket.listen(128)?;
        let listener = TcpListener::from(socket);

        let mut nets = array::from_fn(|_| Self {
            id,
            send: IntMap::default(),
            recv: IntMap::default(),
            timeout,
            max_frame_length,
        });

        for i in 0..N {
            for (other_id, addr) in addrs.iter().enumerate() {
                let addr = addr
                    .to_socket_addrs()?
                    .next()
                    .context("while converting to SocketAddr")?;
                match id.cmp(&other_id) {
                    Ordering::Less => {
                        let stream = connect_with_retry(&addr, timeout)?;
                        let (send_stream, recv_stream) =
                            configure_stream(stream, timeout, i, id)?;
                        nets[i].send.insert(
                            other_id,
                            (
                                Mutex::new(BufWriter::with_capacity(WRITE_BUF_SIZE, send_stream)),
                                AtomicUsize::default(),
                            ),
                        );
                        let (tx, rx) = crossbeam_channel::bounded(CHANNEL_CAP);
                        spawn_reader(recv_stream, max_frame_length, tx);
                        nets[i].recv.insert(other_id, (rx, AtomicUsize::default()));
                    }
                    Ordering::Greater => {
                        let (raw, _) = listener.accept()?;
                        let socket = Socket::from(raw);
                        socket.set_read_timeout(None)?;
                        socket.set_send_buffer_size(SOCKET_BUF_SIZE)?;
                        socket.set_recv_buffer_size(SOCKET_BUF_SIZE)?;
                        let stream = TcpStream::from(socket);
                        let (send_stream, recv_stream, i, other_id) =
                            accept_stream(stream, timeout)?;
                        nets[i].send.insert(
                            other_id,
                            (
                                Mutex::new(BufWriter::with_capacity(WRITE_BUF_SIZE, send_stream)),
                                AtomicUsize::default(),
                            ),
                        );
                        let (tx, rx) = crossbeam_channel::bounded(CHANNEL_CAP);
                        spawn_reader(recv_stream, max_frame_length, tx);
                        nets[i].recv.insert(other_id, (rx, AtomicUsize::default()));
                    }
                    Ordering::Equal => continue,
                }
            }
        }

        Ok(nets)
    }
}

impl Network for FastTcpNetwork {
    fn id(&self) -> usize {
        self.id
    }

    /// Send a frame to party `to`.
    ///
    /// Uses **vectored I/O** (`write_all_vectored`) to merge the 8-byte
    /// length prefix and the payload into a single syscall when possible,
    /// then flushes the `BufWriter` to ensure the data is pushed to the
    /// kernel immediately.
    fn send(&self, to: usize, data: &[u8]) -> eyre::Result<()> {
        if data.len() > self.max_frame_length {
            eyre::bail!("frame len {} > max {}", data.len(), self.max_frame_length);
        }
        let (writer, sent_bytes) = self.send.get(to).context("party id out-of-bounds")?;
        sent_bytes.fetch_add(data.len(), std::sync::atomic::Ordering::Relaxed);

        let len_bytes = (data.len() as u64).to_be_bytes();

        let mut writer = writer.lock();
        // Try vectored I/O first to merge length prefix + payload into one syscall.
        // Fall back to two writes if the OS doesn't support it.
        let bufs = &[IoSlice::new(&len_bytes), IoSlice::new(data)];
        let total = len_bytes.len() + data.len();
        let written = writer.write_vectored(bufs)?;
        if written < total {
            // Partial vectored write – finish the remainder with plain writes.
            if written < len_bytes.len() {
                writer.write_all(&len_bytes[written..])?;
                writer.write_all(data)?;
            } else {
                writer.write_all(&data[written - len_bytes.len()..])?;
            }
        }
        writer.flush()?;
        Ok(())
    }

    fn recv(&self, from: usize) -> eyre::Result<Vec<u8>> {
        let (queue, recv_bytes) = self.recv.get(from).context("party id out-of-bounds")?;
        let data = queue.recv_timeout(self.timeout)??;
        recv_bytes.fetch_add(data.len(), std::sync::atomic::Ordering::Relaxed);
        Ok(data)
    }

    /// Write a frame to party `to` **without flushing**.
    ///
    /// The data is written to the `BufWriter` but not pushed to the kernel.
    /// Call [`flush_to`] or [`flush_all`] to ensure delivery.  This allows
    /// overlapping the flush syscall with a concurrent receive on a
    /// different connection.
    fn send_buffered(&self, to: usize, data: &[u8]) -> eyre::Result<()> {
        if data.len() > self.max_frame_length {
            eyre::bail!("frame len {} > max {}", data.len(), self.max_frame_length);
        }
        let (writer, sent_bytes) = self.send.get(to).context("party id out-of-bounds")?;
        sent_bytes.fetch_add(data.len(), std::sync::atomic::Ordering::Relaxed);

        let len_bytes = (data.len() as u64).to_be_bytes();
        let mut writer = writer.lock();
        let bufs = &[IoSlice::new(&len_bytes), IoSlice::new(data)];
        let total = len_bytes.len() + data.len();
        let written = writer.write_vectored(bufs)?;
        if written < total {
            if written < len_bytes.len() {
                writer.write_all(&len_bytes[written..])?;
                writer.write_all(data)?;
            } else {
                writer.write_all(&data[written - len_bytes.len()..])?;
            }
        }
        // Intentionally NO flush — caller controls when to flush.
        Ok(())
    }

    fn flush_to(&self, to: usize) -> eyre::Result<()> {
        let (writer, _) = self.send.get(to).context("party id out-of-bounds")?;
        writer.lock().flush()?;
        Ok(())
    }

    fn flush_all(&self) -> eyre::Result<()> {
        for (_, (writer, _)) in self.send.iter() {
            writer.lock().flush()?;
        }
        Ok(())
    }

    fn get_connection_stats(&self) -> ConnectionStats {
        let mut stats = std::collections::BTreeMap::new();
        for (id, (_, sent_bytes)) in self.send.iter() {
            let recv_bytes = &self.recv.get(id).expect("was in send so must be in recv").1;
            stats.insert(
                id,
                (
                    sent_bytes.load(std::sync::atomic::Ordering::Relaxed),
                    recv_bytes.load(std::sync::atomic::Ordering::Relaxed),
                ),
            );
        }
        ConnectionStats {
            my_id: self.id,
            stats,
        }
    }
}

// ── Helper functions ──────────────────────────────────────────

/// Connect to `addr` with retry until `timeout`.
fn connect_with_retry(addr: &SocketAddr, timeout: Duration) -> eyre::Result<TcpStream> {
    let start = Instant::now();
    loop {
        if let Ok(stream) = TcpStream::connect_timeout(addr, timeout) {
            return Ok(stream);
        }
        std::thread::sleep(Duration::from_millis(50));
        if start.elapsed() > timeout {
            eyre::bail!("timeout while connecting to {addr}");
        }
    }
}

/// Configure a freshly connected stream (client side):
/// - set large kernel buffers
/// - disable Nagle (nodelay) so BufWriter controls coalescing
/// - write the handshake (network index + party id)
/// - clone for send/recv split
///
/// Returns `(send_half, recv_half)`.
fn configure_stream(
    mut stream: TcpStream,
    timeout: Duration,
    net_index: usize,
    my_id: usize,
) -> eyre::Result<(TcpStream, TcpStream)> {
    // Enlarge kernel buffers on the per-connection socket
    let sock = Socket::from(stream);
    sock.set_send_buffer_size(SOCKET_BUF_SIZE)?;
    sock.set_recv_buffer_size(SOCKET_BUF_SIZE)?;
    stream = TcpStream::from(sock);

    stream.set_write_timeout(Some(timeout))?;
    // nodelay = true: we control batching in userspace via BufWriter
    stream.set_nodelay(true)?;

    // Handshake: tell the server our net_index and party id
    stream.write_u64::<BigEndian>(net_index as u64)?;
    stream.write_u64::<BigEndian>(my_id as u64)?;

    let recv_stream = stream.try_clone().expect("can clone stream");
    Ok((stream, recv_stream))
}

/// Accept side: read the handshake, configure, clone.
/// Returns `(send_half, recv_half, net_index, other_id)`.
fn accept_stream(
    mut stream: TcpStream,
    timeout: Duration,
) -> eyre::Result<(TcpStream, TcpStream, usize, usize)> {
    stream.set_write_timeout(Some(timeout))?;
    stream.set_nodelay(true)?;

    let net_index = stream.read_u64::<BigEndian>()? as usize;
    let other_id = stream.read_u64::<BigEndian>()? as usize;

    let recv_stream = stream.try_clone().expect("can clone stream");
    Ok((stream, recv_stream, net_index, other_id))
}

/// Spawn a dedicated reader thread that wraps the stream in a `BufReader`
/// and pushes frames into the channel.
fn spawn_reader(
    stream: TcpStream,
    max_frame_length: usize,
    tx: crossbeam_channel::Sender<eyre::Result<Vec<u8>>>,
) {
    std::thread::spawn(move || {
        let mut reader = BufReader::with_capacity(READ_BUF_SIZE, stream);
        loop {
            let data = read_next_frame(&mut reader, max_frame_length);
            if tx.send(data).is_err() {
                break;
            }
        }
    });
}

/// Read a single length-prefixed frame from a buffered reader.
fn read_next_frame(
    reader: &mut BufReader<TcpStream>,
    max_frame_length: usize,
) -> eyre::Result<Vec<u8>> {
    let len = reader.read_u64::<BigEndian>()? as usize;
    if len > max_frame_length {
        eyre::bail!("frame len {len} > max {max_frame_length}");
    }
    let mut data = vec![0; len];
    reader.read_exact(&mut data)?;
    Ok(data)
}
