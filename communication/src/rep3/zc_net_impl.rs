use net::Network;

use protocols::rep3_ring::id::PartyID;
use protocols::rep3_ring::ring::int_ring::IntRing2k;
use protocols::rep3_ring::ring::ring_impl::RingElement;
use protocols::rep3_ring::Rep3RingShare;

type RingShare<T> = Rep3RingShare<T>;

/// Data-size threshold (in bytes) above which `send_and_recv` overlaps
/// the flush syscall with the receive using a scoped thread.
///
/// Below this value the sequential path is faster because the
/// `std::thread::scope` spawn overhead (~2-5 µs on Linux) outweighs
/// the flush latency for small payloads.
const CONCURRENT_SEND_RECV_THRESHOLD: usize = 32 * 1024; // 32 KB

// ══════════════════════════════════════════════════════════════
//  ZeroCopy trait — unified interface for zero-copy network I/O
// ══════════════════════════════════════════════════════════════

/// Trait for types that support zero-copy network transmission.
///
/// On little-endian platforms, the in-memory representation matches
/// the wire format, enabling pointer-cast based transmission with
/// zero serialization overhead.
///
/// Implemented for:
/// - [`RingElement<T>`] — `#[repr(transparent)]` newtype over `IntRing2k`
/// - [`Rep3RingShare<T>`] — `#[repr(C)]` struct with two `RingElement<T>`
pub trait ZeroCopy: Sized + Send + Sync {
    /// Reinterpret a slice of `Self` as raw bytes (zero-copy).
    fn slice_as_bytes(slice: &[Self]) -> &[u8];
    /// Reconstruct a `Vec<Self>` from raw bytes (zero-copy).
    fn vec_from_bytes(bytes: Vec<u8>) -> eyre::Result<Vec<Self>>;
}

impl<T: IntRing2k> ZeroCopy for RingElement<T> {
    #[inline(always)]
    fn slice_as_bytes(slice: &[Self]) -> &[u8] {
        RingElement::slice_as_bytes(slice)
    }

    #[inline(always)]
    fn vec_from_bytes(bytes: Vec<u8>) -> eyre::Result<Vec<Self>> {
        RingElement::vec_from_bytes(bytes)
    }
}

impl<T: IntRing2k> ZeroCopy for RingShare<T> {
    #[inline(always)]
    fn slice_as_bytes(slice: &[Self]) -> &[u8] {
        RingShare::slice_as_bytes(slice)
    }

    #[inline(always)]
    fn vec_from_bytes(bytes: Vec<u8>) -> eyre::Result<Vec<Self>> {
        RingShare::vec_from_bytes(bytes)
    }
}

/// An extension trait that adds REP3 specific methods to [`Network`].
pub trait Rep3ZeroCopyNetworkImpl: Network {

    // ══════════════════════════════════════════════════════════════
    //  Zero-copy methods  (`_zc` suffix)
    //
    //  Works with any type implementing `ZeroCopy`:
    //  - `RingElement<T>` — #[repr(transparent)] newtype, pointer cast
    //  - `Rep3RingShare<T>` — #[repr(C)], two RingElements
    // ══════════════════════════════════════════════════════════════

    /// Send a slice of `Z` to the target party (zero-copy).
    #[inline(always)]
    fn send_many_zc<Z: ZeroCopy>(
        &self,
        to: PartyID,
        data: &[Z],
    ) -> eyre::Result<()> {
        let bytes = Z::slice_as_bytes(data);
        self.send(to.into(), bytes)?;
        Ok(())
    }

    /// Receive a `Vec<Z>` from a party (zero-copy).
    #[inline(always)]
    fn recv_many_zc<Z: ZeroCopy>(
        &self,
        from: PartyID,
    ) -> eyre::Result<Vec<Z>> {
        let bytes = self.recv(from.into())?;
        Z::vec_from_bytes(bytes)
    }

    /// Send a single `Z` to the target party (zero-copy).
    #[inline(always)]
    fn send_to_zc<Z: ZeroCopy>(
        &self,
        to: PartyID,
        data: Z,
    ) -> eyre::Result<()> {
        self.send_many_zc(to, &[data])
    }

    /// Receive a single `Z` from a party (zero-copy).
    #[inline(always)]
    fn recv_from_zc<Z: ZeroCopy>(
        &self,
        from: PartyID,
    ) -> eyre::Result<Z> {
        let mut res = self.recv_many_zc(from)?;
        if res.len() != 1 {
            eyre::bail!("Expected 1 element, got {}", res.len())
        }
        Ok(res.pop().unwrap())
    }

    /// Send a single `Z` to next party (zero-copy).
    #[inline(always)]
    fn send_next_zc<Z: ZeroCopy>(
        &self,
        data: Z,
    ) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_to_zc(id.next(), data)
    }

    /// Send a slice of `Z` to next party (zero-copy).
    #[inline(always)]
    fn send_next_many_zc<Z: ZeroCopy>(
        &self,
        data: &[Z],
    ) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_many_zc(id.next(), data)
    }

    /// Send a single `Z` to prev party (zero-copy).
    #[inline(always)]
    fn send_prev_zc<Z: ZeroCopy>(
        &self,
        data: Z,
    ) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_to_zc(id.prev(), data)
    }

    /// Send a slice of `Z` to prev party (zero-copy).
    #[inline(always)]
    fn send_prev_many_zc<Z: ZeroCopy>(
        &self,
        data: &[Z],
    ) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_many_zc(id.prev(), data)
    }

    /// Receive a single `Z` from next party (zero-copy).
    #[inline(always)]
    fn recv_next_zc<Z: ZeroCopy>(&self) -> eyre::Result<Z> {
        let id = PartyID::try_from(self.id())?;
        self.recv_from_zc(id.next())
    }

    /// Receive a `Vec<Z>` from next party (zero-copy).
    #[inline(always)]
    fn recv_next_many_zc<Z: ZeroCopy>(&self) -> eyre::Result<Vec<Z>> {
        let id = PartyID::try_from(self.id())?;
        self.recv_many_zc(id.next())
    }

    /// Receive a single `Z` from prev party (zero-copy).
    #[inline(always)]
    fn recv_prev_zc<Z: ZeroCopy>(&self) -> eyre::Result<Z> {
        let id = PartyID::try_from(self.id())?;
        self.recv_from_zc(id.prev())
    }

    /// Receive a `Vec<Z>` from prev party (zero-copy).
    #[inline(always)]
    fn recv_prev_many_zc<Z: ZeroCopy>(&self) -> eyre::Result<Vec<Z>> {
        let id = PartyID::try_from(self.id())?;
        self.recv_many_zc(id.prev())
    }

    /// Reshare a single `Z`: send to next, receive from prev (zero-copy).
    #[inline(always)]
    fn reshare_zc<Z: ZeroCopy>(
        &self,
        data: Z,
    ) -> eyre::Result<Z> {
        let mut res = self.reshare_many_zc(&[data])?;
        if res.len() != 1 {
            eyre::bail!("Expected 1 element, got {}", res.len())
        }
        Ok(res.pop().unwrap())
    }

    /// Reshare a slice of `Z` in one round (zero-copy).
    #[inline(always)]
    fn reshare_many_zc<Z: ZeroCopy>(
        &self,
        data: &[Z],
    ) -> eyre::Result<Vec<Z>> {
        let id = PartyID::try_from(self.id())?;
        self.send_and_recv_many_zc(id.next(), data, id.prev())
    }

    /// Broadcast a single `Z` and receive from both parties (zero-copy).
    #[inline(always)]
    fn broadcast_zc<Z: ZeroCopy>(
        &self,
        data: Z,
    ) -> eyre::Result<(Z, Z)> {
        let (mut prev, mut next) = self.broadcast_many_zc(&[data])?;
        if prev.len() != 1 || next.len() != 1 {
            eyre::bail!("Expected 1 element, got more")
        }
        Ok((prev.pop().unwrap(), next.pop().unwrap()))
    }

    /// Broadcast a slice of `Z` and receive from both parties (zero-copy).
    #[inline(always)]
    fn broadcast_many_zc<Z: ZeroCopy>(
        &self,
        data: &[Z],
    ) -> eyre::Result<(Vec<Z>, Vec<Z>)> {
        let id = PartyID::try_from(self.id())?;
        let next_id = id.next();
        let prev_id = id.prev();
        let (prev_res, next_res) = net::join(
            || self.send_and_recv_many_zc(prev_id, data, prev_id),
            || self.send_and_recv_many_zc(next_id, data, next_id),
        );
        Ok((prev_res?, next_res?))
    }

    /// Send and receive a single `Z` to/from different parties (zero-copy).
    #[inline(always)]
    fn send_and_recv_zc<Z: ZeroCopy>(
        &self,
        to: PartyID,
        data: Z,
        from: PartyID,
    ) -> eyre::Result<Z> {
        let mut res = self.send_and_recv_many_zc(to, &[data], from)?;
        if res.len() != 1 {
            eyre::bail!("Expected 1 element, got {}", res.len())
        }
        Ok(res.pop().unwrap())
    }

    /// Send and receive slices of `Z` to/from different parties (zero-copy).
    ///
    /// For payloads ≥ 32 KB, overlaps the flush syscall with the receive
    /// using a scoped thread — this hides the memcpy-to-kernel latency
    /// behind the network wait time.  For smaller payloads the sequential
    /// path is used to avoid thread-spawn overhead.
    #[inline(always)]
    fn send_and_recv_many_zc<Z: ZeroCopy>(
        &self,
        to: PartyID,
        data: &[Z],
        from: PartyID,
    ) -> eyre::Result<Vec<Z>> {
        let data_bytes = std::mem::size_of::<Z>() * data.len();
        if data_bytes >= CONCURRENT_SEND_RECV_THRESHOLD {
            // Large payload: write without flushing, then overlap
            // flush with recv on separate connections.
            let bytes = Z::slice_as_bytes(data);
            self.send_buffered(to.into(), bytes)?;
            let to_usize: usize = to.into();
            let (flush_res, recv_res) = net::join(
                || self.flush_to(to_usize),
                || self.recv_many_zc(from),
            );
            flush_res?;
            recv_res
        } else {
            // Small payload: sequential (avoid thread-spawn overhead).
            self.send_many_zc(to, data)?;
            self.recv_many_zc(from)
        }
    }
}


impl<N: Network> Rep3ZeroCopyNetworkImpl for N {}

