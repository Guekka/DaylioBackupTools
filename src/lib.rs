#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
// for tests
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_wrap)]

pub use daylio::*;
pub use load_store::*;
pub use merge::merge;

mod analyze_pdf;
mod daylio;
mod load_store;
mod merge;
mod models;
mod parse_md;
mod parse_pdf;
