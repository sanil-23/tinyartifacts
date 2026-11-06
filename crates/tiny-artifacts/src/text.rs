//! Reading a file artifact as text.
//!
//! Decoding a PDF, a DOCX or a PPTX takes a document library this crate does
//! not carry, so the host supplies it. Text kinds need none of this: their
//! content already is text.

use async_trait::async_trait;

/// Turns a file's bytes into the text a model can read.
#[async_trait]
pub trait TextExtractor: Send + Sync {
    /// The text of the file `name`, or `None` for a format with no text to read.
    ///
    /// # Errors
    ///
    /// When the format is readable but this file could not be read.
    async fn extract(&self, name: &str, bytes: &[u8]) -> anyhow::Result<Option<String>>;
}
