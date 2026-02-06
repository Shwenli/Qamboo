//! # Qamboo
//!
//! This is the main library entry point for the project (Library Root).
//!
//! In Rust, this file is used to define the public API structure of the library.
//! The `pub mod` declarations below declare the top-level modules contained in the project.
//! 
//! When external code references this crate (e.g., `use protocols;`),
//! it finds the corresponding code through these declarations.

// -----------------------------------------------------------------------------
// Module Declarations
// -----------------------------------------------------------------------------

/// Secure Multiparty Computation Protocol Implementation Module (Protocols)
/// Contains low-level MPC protocol logic.
pub mod protocols;

/// Cryptographic Primitives Module (Primitives)
/// Provides basic building blocks such as compare, shuffle, permute, etc.
pub mod primitives;

/// Operators Module (Operators)
/// Implements database-style high-level operators, such as Join, GroupBy, Sort, etc.
pub mod operator;

/// Table Structure Module (Table)
/// Defines data structures for secret sharing tables (ShareTable) and columns (ShareColumn).
/// Also from implementations of various operators to High-level API  on these structures. 
pub mod table;

/// Network Communication Module (Network)
/// Responsible for TCP communication and data transmission between nodes.
pub mod net;


