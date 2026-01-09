extern crate rand;
#[macro_use]
extern crate smallvec;
#[macro_use]
extern crate serde_derive;
extern crate serde;

pub mod mechanics;

#[cfg(feature = "tui-widgets")]
mod tui;
#[cfg(feature = "tui-widgets")]
pub use crate::tui::*;
