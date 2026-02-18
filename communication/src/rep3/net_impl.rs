//! Rep3 Network
//!
//! This module contains the networking functionality for the Rep3 MPC protocol.
//!


use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use net::Network;

use crate::rep3::id::PartyID;

/// Data-size threshold (in bytes) above which `send_and_recv` overlaps
/// the flush syscall with the receive using a scoped thread.
///
/// Below this value the sequential path is faster because the
/// `std::thread::scope` spawn overhead (~2-5 µs on Linux) outweighs
/// the flush latency for small payloads.
const CONCURRENT_SEND_RECV_THRESHOLD: usize = 32 * 1024; // 32 KB


/// An extension trait that adds REP3 specific methods to [`Network`].
pub trait Rep3NetworkImpl: Network {

    // ══════════════════════════════════════════════════════════════
    //  Serialization methods (generic F, no suffix)
    //
    //  Optimized over naive ark_serialize:
    //  - `send_many`: skips `serialized_size()` iteration by using
    //    `serialize_uncompressed` directly into a pre-grown Vec via
    //    Cursor, avoiding double traversal.
    //  - `recv_many`: uses `deserialize_uncompressed_unchecked` which
    //    skips validation (safe for trusted MPC peers).
    //  - For types with a **fixed** serialized size (like RingElement),
    //    `send_many_fast` / `recv_many_fast` provide an even faster
    //    path using bulk memcpy.
    // ══════════════════════════════════════════════════════════════

    /// Send a single `F` directly serialized to the network.
    ///
    /// Optimization: uses `std::mem::size_of::<F>()` as capacity hint
    /// instead of calling `serialized_size()`. For fixed-layout MPC types
    /// (RingElement, tuples, etc.), `size_of == serialized_size`, so the
    /// Vec is perfectly sized with **zero reallocation**.
    #[inline(always)]
    fn send_to<F: CanonicalSerialize>(
        &self,
        to: PartyID,
        data: F,
    ) -> eyre::Result<()> {
        // size_of is a compile-time constant — no traversal!
        let mut buf = Vec::with_capacity(std::mem::size_of::<F>().max(64));
        data.serialize_uncompressed(&mut buf)?;
        self.send(to.into(), &buf)?;
        Ok(())
    }

    /// Receive a single `F` via `CanonicalDeserialize`.
    #[inline(always)]
    fn recv_from<F: CanonicalDeserialize>(
        &self,
        from: PartyID,
    ) -> eyre::Result<F> {
        let data = self.recv(from.into())?;
        let res = F::deserialize_uncompressed_unchecked(&data[..])?;
        Ok(res)
    }

    /// Send a slice of `F` – optimized to avoid double-traversal.
    ///
    /// **Key optimization**: uses `std::mem::size_of::<F>() * len` as
    /// capacity hint instead of calling `serialized_size()` which
    /// iterates *all* N elements.  For fixed-layout types this gives
    /// an exact allocation; for variable-size types the Vec will grow
    /// as needed (rare in MPC).
    #[inline(always)]
    fn send_many<F: CanonicalSerialize>(
        &self,
        to: PartyID,
        data: &[F],
    ) -> eyre::Result<()> {
        // Heuristic capacity: 8 (u64 length prefix) + size_of<F> * N.
        // Compile-time constant per-element; no O(N) serialized_size().
        let hint = 8 + std::mem::size_of::<F>() * data.len();
        let mut ser_data = Vec::with_capacity(hint.max(64));
        data.serialize_uncompressed(&mut ser_data)?;
        self.send(to.into(), &ser_data)?;
        Ok(())
    }

    /// Receive a `Vec<F>` via `CanonicalDeserialize`.
    #[inline(always)]
    fn recv_many<F: CanonicalDeserialize>(
        &self,
        from: PartyID,
    ) -> eyre::Result<Vec<F>> {
        let data = self.recv(from.into())?;
        let res = Vec::<F>::deserialize_uncompressed_unchecked(&data[..])?;
        Ok(res)
    }

    /// High-performance send for fixed-size `CanonicalSerialize` types.
    ///
    /// Computes the exact buffer size from the first element's
    /// `serialized_size()` — a **single** call instead of N calls.
    /// Then manually writes the u64 length prefix and per-element data
    /// into the pre-sized buffer.
    ///
    /// Advantage over `send_many`: avoids the `size_of` heuristic and
    /// guarantees **zero Vec reallocation** for any fixed-size type,
    /// even if `size_of != serialized_size` (e.g. types with padding).
    #[inline(always)]
    fn send_many_fast<F: CanonicalSerialize>(
        &self,
        to: PartyID,
        data: &[F],
    ) -> eyre::Result<()> {
        if data.is_empty() {
            // Still need to send the empty-slice marker (8-byte 0)
            self.send(to.into(), &0u64.to_le_bytes())?;
            return Ok(());
        }
        // Compute element size from first element (constant for fixed-size types)
        let elem_size = data[0].serialized_size(ark_serialize::Compress::No);
        let total = 8 + elem_size * data.len(); // 8 bytes for u64 length prefix
        let mut buf = Vec::with_capacity(total);
        // Write element count (matches ark's Vec<T> serialization format)
        buf.extend_from_slice(&(data.len() as u64).to_le_bytes());
        // Serialize elements directly; for RingElement this is just to_le_bytes
        for item in data {
            item.serialize_uncompressed(&mut buf)?;
        }
        debug_assert_eq!(buf.len(), total);
        self.send(to.into(), &buf)?;
        Ok(())
    }

    /// High-performance receive for fixed-size types.
    ///
    /// Paired with `send_many_fast`. Uses `deserialize_uncompressed_unchecked`
    /// which skips validity checks (safe for trusted MPC communication).
    #[inline(always)]
    fn recv_many_fast<F: CanonicalDeserialize>(
        &self,
        from: PartyID,
    ) -> eyre::Result<Vec<F>> {
        let data = self.recv(from.into())?;
        let res = Vec::<F>::deserialize_uncompressed_unchecked(&data[..])?;
        Ok(res)
    }

    // ── Fast convenience wrappers ──────────────────────────────────

    /// Send a slice of `F` to next party (fast path for fixed-size types).
    #[inline(always)]
    fn send_next_many_fast<F: CanonicalSerialize>(&self, data: &[F]) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_many_fast(id.next(), data)
    }

    /// Send a slice of `F` to prev party (fast path for fixed-size types).
    #[inline(always)]
    fn send_prev_many_fast<F: CanonicalSerialize>(&self, data: &[F]) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_many_fast(id.prev(), data)
    }

    /// Receive a `Vec<F>` from next party (fast path for fixed-size types).
    #[inline(always)]
    fn recv_next_many_fast<F: CanonicalDeserialize>(&self) -> eyre::Result<Vec<F>> {
        let id = PartyID::try_from(self.id())?;
        self.recv_many_fast(id.next())
    }

    /// Receive a `Vec<F>` from prev party (fast path for fixed-size types).
    #[inline(always)]
    fn recv_prev_many_fast<F: CanonicalDeserialize>(&self) -> eyre::Result<Vec<F>> {
        let id = PartyID::try_from(self.id())?;
        self.recv_many_fast(id.prev())
    }

    /// Reshare a slice of `F` in one round (fast path for fixed-size types).
    #[inline(always)]
    fn reshare_many_fast<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        data: &[F],
    ) -> eyre::Result<Vec<F>> {
        let id = PartyID::try_from(self.id())?;
        self.send_many_fast(id.next(), data)?;
        self.recv_many_fast(id.prev())
    }

    /// Broadcast a slice of `F` and receive from both parties (fast path).
    #[inline(always)]
    fn broadcast_many_fast<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        data: &[F],
    ) -> eyre::Result<(Vec<F>, Vec<F>)> {
        let id = PartyID::try_from(self.id())?;
        let next_id = id.next();
        let prev_id = id.prev();
        let (prev_res, next_res) = net::join(
            || {
                self.send_many_fast(prev_id, data)?;
                self.recv_many_fast::<F>(prev_id)
            },
            || {
                self.send_many_fast(next_id, data)?;
                self.recv_many_fast::<F>(next_id)
            },
        );
        Ok((prev_res?, next_res?))
    }

    // ── Serialized convenience wrappers ────────────────────────────

    /// Send a single `F` to next party via `CanonicalSerialize`.
    #[inline(always)]
    fn send_next<F: CanonicalSerialize>(&self, data: F) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_to(id.next(), data)
    }

    /// Send a slice of `F` to next party via `CanonicalSerialize`.
    #[inline(always)]
    fn send_next_many<F: CanonicalSerialize>(&self, data: &[F]) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_many(id.next(), data)
    }

    /// Send a single `F` to prev party via `CanonicalSerialize`.
    #[inline(always)]
    fn send_prev<F: CanonicalSerialize>(&self, data: F) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_to(id.prev(), data)
    }

    /// Send a slice of `F` to prev party via `CanonicalSerialize`.
    #[inline(always)]
    fn send_prev_many<F: CanonicalSerialize>(&self, data: &[F]) -> eyre::Result<()> {
        let id = PartyID::try_from(self.id())?;
        self.send_many(id.prev(), data)
    }

    /// Receive a single `F` from next party via `CanonicalDeserialize`.
    #[inline(always)]
    fn recv_next<F: CanonicalDeserialize>(&self) -> eyre::Result<F> {
        let id = PartyID::try_from(self.id())?;
        self.recv_from(id.next())
    }

    /// Receive a `Vec<F>` from next party via `CanonicalDeserialize`.
    #[inline(always)]
    fn recv_next_many<F: CanonicalDeserialize>(&self) -> eyre::Result<Vec<F>> {
        let id = PartyID::try_from(self.id())?;
        self.recv_many(id.next())
    }

    /// Receive a single `F` from prev party via `CanonicalDeserialize`.
    #[inline(always)]
    fn recv_prev<F: CanonicalDeserialize>(&self) -> eyre::Result<F> {
        let id = PartyID::try_from(self.id())?;
        self.recv_from(id.prev())
    }

    /// Receive a `Vec<F>` from prev party via `CanonicalDeserialize`.
    #[inline(always)]
    fn recv_prev_many<F: CanonicalDeserialize>(&self) -> eyre::Result<Vec<F>> {
        let id = PartyID::try_from(self.id())?;
        self.recv_many(id.prev())
    }

    /// Reshare a single `F`: send to next, receive from prev (serialized).
    #[inline(always)]
    fn reshare<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        data: F,
    ) -> eyre::Result<F> {
        let mut res = self.reshare_many(&[data])?;
        if res.len() != 1 {
            eyre::bail!("Expected 1 element, got more")
        }
        Ok(res.pop().unwrap())
    }

    /// Reshare a slice of `F` in one round (serialized).
    #[inline(always)]
    fn reshare_many<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        data: &[F],
    ) -> eyre::Result<Vec<F>> {
        let id = PartyID::try_from(self.id())?;
        self.send_and_recv_many(id.next(), data, id.prev())
    }

    /// Broadcast a single `F` and receive from both parties (serialized).
    #[inline(always)]
    fn broadcast<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        data: F,
    ) -> eyre::Result<(F, F)> {
        let (mut prev, mut next) = self.broadcast_many(&[data])?;
        if prev.len() != 1 || next.len() != 1 {
            eyre::bail!("Expected 1 element, got more")
        }
        let prev = prev.pop().unwrap();
        let next = next.pop().unwrap();
        Ok((prev, next))
    }

    /// Broadcast a slice of `F` and receive from both parties (serialized).
    #[inline(always)]
    fn broadcast_many<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        data: &[F],
    ) -> eyre::Result<(Vec<F>, Vec<F>)> {
        let id = PartyID::try_from(self.id())?;
        let next_id = id.next();
        let prev_id = id.prev();
        let (prev_res, next_res) = net::join(
            || self.send_and_recv_many(prev_id, data, prev_id),
            || self.send_and_recv_many(next_id, data, next_id),
        );
        Ok((prev_res?, next_res?))
    }

    /// Send and receive a single `F` to/from parties (serialized).
    ///
    /// Uses `send_to` + `recv_from` (single-element format) so that
    /// the wire format is compatible with `send_next` / `recv_prev`
    /// which also use the single-element format.
    #[inline(always)]
    fn send_and_recv<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        to: PartyID,
        data: F,
        from: PartyID,
    ) -> eyre::Result<F> {
        self.send_to(to, data)?;
        self.recv_from(from)
    }

    /// Send and receive slices of `F` to/from parties (serialized).
    ///
    /// For large payloads, overlaps the flush with the receive.
    #[inline(always)]
    fn send_and_recv_many<F: CanonicalSerialize + CanonicalDeserialize + Send>(
        &self,
        to: PartyID,
        data: &[F],
        from: PartyID,
    ) -> eyre::Result<Vec<F>> {
        // Serialize into a byte buffer first.
        let hint = 8 + std::mem::size_of::<F>() * data.len();
        let mut ser_data = Vec::with_capacity(hint.max(64));
        data.serialize_uncompressed(&mut ser_data)?;

        if ser_data.len() >= CONCURRENT_SEND_RECV_THRESHOLD {
            self.send_buffered(to.into(), &ser_data)?;
            let to_usize: usize = to.into();
            let (flush_res, recv_res) = net::join(
                || self.flush_to(to_usize),
                || self.recv_many(from),
            );
            flush_res?;
            recv_res
        } else {
            self.send(to.into(), &ser_data)?;
            self.recv_many(from)
        }
    }
}

impl<N: Network> Rep3NetworkImpl for N {}
