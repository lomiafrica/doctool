//! Portable code intelligence modules copied from Composer `code_intel`.
//! COMPOSER_SOURCE: composer/src-tauri/src/code_intel/

pub mod buffer;
pub mod embedder;
pub mod global_index;
pub mod graph;
pub mod indexer;
pub mod language_detect;
pub mod loader;
pub mod parser;
pub mod search;
pub mod types;
pub mod utils;
pub mod vector_store;

pub use types::*;
