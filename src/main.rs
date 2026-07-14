mod app;
mod components;
mod difficulty;
mod game;
mod input;
mod progress;
mod reddit;
mod storage;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! { <App /> }
    });
}
