#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

pub mod config;
pub mod errors;
pub mod identities;
pub mod projection;
pub mod record;

#[cfg(target_arch = "wasm32")]
pub mod browser;
#[cfg(target_arch = "wasm32")]
pub mod client;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
