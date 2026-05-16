//! Contains the program state and UI

mod board;

use board::DcaAssignable;
use crate::dB;
use eframe::egui;

/// Top level of program state
#[derive(Default)]
pub struct State {
    // Actual program state
    cues: Vec<Cue>,

    // UI state etc
    ui_screen: UiScreen,
}

impl State {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self::default()
    }
}

impl eframe::App for State {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Create the top menu bar and draw its buttons
            ui.horizontal(|ui| {
                ui.heading("miq-v2");
                ui.selectable_value(&mut self.ui_screen, UiScreen::Cues, "Cues");
                ui.selectable_value(&mut self.ui_screen, UiScreen::File, "File");
                ui.selectable_value(&mut self.ui_screen, UiScreen::Board, "Board");
            });
            ui.heading("Goodbye, World.");
        });
   }
}

/// The different screens of the ui, like the cues, board connection, etc
#[derive(Default, PartialEq)]
enum UiScreen {
    #[default]
    Cues,
    File,
    Board,
}

/// One singular cue aka scene
struct Cue {
    dcas: Vec<DcaState>,
}

/// The state of a DCA, which can be realized by calling a cue
struct DcaState {
    assigned: Vec<Box<dyn DcaAssignable>>,
    level: Option<dB>,
}
