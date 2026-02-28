#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
// for tests
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_wrap)]

pub use formats::daylio::*;
pub use formats::load_store::*;
pub use formats::models::*;
pub use tools::merge::*;

mod formats;
pub mod tools;
