//! A simple networking layer for MPC protocols.
#![warn(missing_docs)]
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

pub mod config;

#[cfg(feature = "fast_tcp")]
pub mod fast_tcp;

#[cfg(feature = "tcp")]
pub mod tcp;





const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_FRAME_LENTH: usize = 128 * 1024 * 1024; // 128MB

/// A MPC network that can be used to send and receive data to and from other parties
///
/// Can be used to send to multiple parties in parallel, but sending to the same party must happen in sequence.
pub trait Network: Send + Sync {
    /// The id of the party
    fn id(&self) -> usize;
    /// Send data to other party
    fn send(&self, to: usize, data: &[u8]) -> eyre::Result<()>;
    /// Receive data from other party
    fn recv(&self, from: usize) -> eyre::Result<Vec<u8>>;

    /// Get connection statistics for the Network.
    /// The returned HashMap maps party_id to a tuple of (sent_bytes, received_bytes).
    fn get_connection_stats(&self) -> ConnectionStats;

    /// Write data to the send buffer without forcing immediate transmission.
    ///
    /// The data will be delivered when [`flush_to`] or [`flush_all`] is
    /// called, or when the internal buffer is full.  Implementations
    /// without userspace buffering (like [`crate::tcp::TcpNetwork`])
    /// simply delegate to [`send`].
    ///
    /// Default: delegates to [`send`].
    fn send_buffered(&self, to: usize, data: &[u8]) -> eyre::Result<()> {
        self.send(to, data)
    }

    /// Flush any buffered data for the connection to party `to`.
    ///
    /// After this call, all data previously written via [`send_buffered`]
    /// to party `to` is guaranteed to have been pushed to the kernel.
    ///
    /// Default: no-op (for unbuffered implementations).
    fn flush_to(&self, _to: usize) -> eyre::Result<()> {
        Ok(())
    }

    /// Flush buffered data for **all** connections.
    ///
    /// Default: no-op.
    fn flush_all(&self) -> eyre::Result<()> {
        Ok(())
    }
}

// This implements a dummy network that is used for plain variants of MPC protocols
impl Network for () {
    fn id(&self) -> usize {
        0
    }

    fn send(&self, _to: usize, _data: &[u8]) -> eyre::Result<()> {
        Ok(())
    }

    fn recv(&self, _from: usize) -> eyre::Result<Vec<u8>> {
        Ok(vec![])
    }

    fn get_connection_stats(&self) -> ConnectionStats {
        ConnectionStats {
            my_id: 0,
            stats: BTreeMap::new(),
        }
    }
}

/// Statistics about the number of bytes sent over the network.
pub struct ConnectionStats {
    my_id: usize,
    stats: BTreeMap<usize, (usize, usize)>,
}

impl ConnectionStats {
    /// Get connection statistics for a specific party.
    /// Returns a tuple of (sent_bytes, received_bytes) if the party_id exists, otherwise returns None.
    pub fn get(&self, party_id: usize) -> Option<(usize, usize)> {
        self.stats.get(&party_id).cloned()
    }

    /// Get an iterator over the connection statistics.
    /// Iterates over the parties in ascending order of their IDs.
    pub fn iter(&self) -> impl Iterator<Item = (usize, (usize, usize))> {
        self.stats.iter().map(|(&id, &stats)| (id, stats))
    }

    /// Get connection statistics for a given time period by calculating the difference between two ConnectionStats instances.
    pub fn get_diff_to(&self, other: &ConnectionStats) -> HashMap<usize, (usize, usize)> {
        let mut diff = HashMap::new();
        for (&id, &(sent, recv)) in &self.stats {
            if let Some(&(other_sent, other_recv)) = other.stats.get(&id) {
                diff.insert(id, (sent - other_sent, recv - other_recv));
            } else {
                diff.insert(id, (sent, recv));
            }
        }
        diff
    }
}

impl std::fmt::Display for ConnectionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (id, (sent, recv)) in self.iter() {
            writeln!(
                f,
                "Party {my_id} <-> {id}: SENT {sent} bytes, RECV {recv} bytes",
                my_id = self.my_id
            )?;
        }
        Ok(())
    }
}

/// Run 2 network closures
#[inline(always)]
pub fn join<R0: Send, R1: Send>(
    f0: impl FnOnce() -> R0 + Send,
    f1: impl FnOnce() -> R1 + Send,
) -> (R0, R1) {
    std::thread::scope(|s| {
        let r0 = s.spawn(f0);
        let r1 = f1();
        (r0.join().expect("can join"), r1)
    })
}


/// Run a variable number of network closures
pub fn join_all<I, F, R>(iter: I) -> Vec<R>
where
    I: IntoIterator<Item = F>,
    F: FnOnce() -> R + Send,
    R: Send,
{
    let mut iter = iter.into_iter();
    
    // 1. 尝试取出第一个任务
    let first_func = match iter.next() {
        Some(f) => f,
        None => return Vec::new(),
    };

    std::thread::scope(|s| {
        // 2. 将剩余的任务直接 spawn，只收集句柄 (Handle 比 Closure 小得多)
        // 这样避免了分配一个包含所有 Closure 的大 Vec
        let handles: Vec<_> = iter.map(|f| s.spawn(f)).collect();

        // 3. 在当前线程执行第一个任务 (利用当前线程，减少一次上下文切换)
        let first_res = first_func();

        // 4. 预分配结果 Vec
        let mut results = Vec::with_capacity(handles.len() + 1);
        
        // 5. 按顺序推入结果
        results.push(first_res); // 第一个结果
        for h in handles {
            results.push(h.join().expect("can join")); // 剩余结果
        }
        
        results
    })
}
