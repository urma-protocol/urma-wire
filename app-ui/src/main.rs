#![deny(unused_must_use)]
#![deny(for_loops_over_fallibles)]
#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_assignments)]

use leptos::mount::mount_to_body;
use wire_app_ui::app::App;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
