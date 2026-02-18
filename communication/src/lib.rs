//! # Naming convention
//!
//! - **`_zc` suffix**: zero-copy methods for types implementing [`ZeroCopy`].
//!   Bypass `ark_serialize` entirely and transmute slices to/from raw
//!   bytes.  On little-endian platforms this is a pointer cast –
//!   zero allocation, zero per-element work.
//!   Supported types: `RingElement<T>`, `Rep3RingShare<T>`.
//!
//! - **No suffix** (default): generic serialized methods via
//!   `CanonicalSerialize` / `CanonicalDeserialize`.  Use these for
//!   types that need actual serialization (seeds, tuples, arrays, etc.).
//!   Optimized: uses `size_of::<F>()` heuristic capacity to avoid
//!   the O(N) `serialized_size()` traversal.
//!
//! - **`_fast` suffix**: specialized serialized methods for **fixed-size**
//!   `CanonicalSerialize` types.  Computes exact buffer size from a
//!   single element's `serialized_size()`, then writes the length
//!   prefix + per-element data in one pass.  Zero Vec reallocation
//!   guaranteed.
//!
//! # Performance hierarchy (fastest → slowest)
//!
//! 1. `_zc` — zero-copy pointer cast, ~0 overhead
//! 2. `_fast` — exact allocation, 1-pass serialization
//! 3. default (no suffix) — heuristic allocation, ark serialization

pub mod rep3;
pub mod task;


