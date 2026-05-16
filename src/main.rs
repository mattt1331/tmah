mod app;
mod util;

pub use util::dB;

use app::State;

fn main() -> eframe::Result {
    // Configure eframe and run app
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("miq-v2", native_options, Box::new(|creation_context| Ok(Box::new(State::new(creation_context)))))
}
