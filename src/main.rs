mod app;
mod util;

pub use util::dB;

use app::State;

// When compiling natively
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    // Enable logging to stderr
    env_logger::init();

    // Configure eframe and run app
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "miq-v2",
        native_options,
        Box::new(|creation_context| Ok(Box::new(State::new(creation_context)))),
    )
}

// When compiling for web
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast; /*as _*/
    use eframe::web_sys;

    // Enable logging to the web console
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    // Configure eframe and run web app
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(State::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
