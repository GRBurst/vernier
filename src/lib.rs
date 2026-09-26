//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The pure core: prose extraction, word counting and source positions.
//! All I/O lives in `main.rs` (ENGINEERING.md §6).

pub mod block;
pub mod cli;
pub mod position;
pub mod prose;
pub mod readability;
pub mod summary;
pub mod syllables;
pub mod words;
