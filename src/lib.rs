//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The pure core: prose extraction, word counting and source positions.
//! All I/O lives in `main.rs` (ENGINEERING.md §6).

pub mod analysis;
pub mod block;
pub mod cli;
pub mod dependency;
pub mod position;
pub mod prose;
pub mod readability;
pub mod sentence;
pub mod summary;
pub mod syllables;
pub mod syntax;
#[cfg(test)]
mod testing;
pub mod words;
