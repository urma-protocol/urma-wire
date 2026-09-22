#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

pub mod app;
pub mod components;
pub mod data;
pub mod limits;
pub mod pages;
pub mod platform;
pub mod state;

pub use app::App;
