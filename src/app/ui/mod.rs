//! Contains all the UI

use super::{State, UiScreen};
use eframe::egui::{self, ScrollArea};
use egui_extras::{Column, TableBuilder};

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
            ui.add_space(10.0);
            // Draw the main area
            match self.ui_screen {
                UiScreen::Cues => {
                    self.cues_ui(ui);
                }
                UiScreen::File => {
                    ui.heading("File");
                }
                UiScreen::Board => {
                    ui.heading("Board");
                }
            }
        });
    }
}

/// UI
impl State {
    /// Draw the UI of the area that shows the cues and DCAs
    fn cues_ui(&mut self, ui: &mut egui::Ui) {
        const HEADER_HEIGHT: f32 = 20.0;
        const ROW_HEIGHT: f32 = 20.0;
        ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            TableBuilder::new(ui)
                // All columns must be "pre-allocated"
                .column(Column::auto())
                .columns(Column::auto(), self.num_dcas().into())
                .header(HEADER_HEIGHT, |mut header| {
                    header.col(|ui| {
                        ui.heading("Cue");
                    });
                    for i in 0..self.num_dcas() {
                        header.col(|ui| {
                            ui.heading(format!("DCA {}", i + 1));
                        });
                    }
                })
                .body(|mut body| {
                    for cue in self.cues() {
                        body.row(ROW_HEIGHT, |mut row| {
                            row.col(|ui| {
                                ui.label(cue.name());
                            });
                            for dca in cue.dcas() {
                                row.col(|ui| {
                                    ui.label(dca.name());
                                });
                            }
                        })
                    }
                })
        });
    }
}
