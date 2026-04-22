//! PDF export for RaTeX [`DisplayList`](ratex_types::display_item::DisplayList).
//!
//! Built directly on [pdf-writer](https://docs.rs/pdf-writer) with manual font subsetting
//! via [subsetter](https://docs.rs/subsetter). Produces compact PDFs with embedded CIDFontType2
//! fonts and `Identity-H` encoding. No high-level abstraction layer.

mod fonts;
mod renderer;

pub use renderer::{render_to_pdf, PdfError, PdfOptions};
