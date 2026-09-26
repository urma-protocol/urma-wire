#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

fn main() {
    wire_app_desktop::shell::run();
}
