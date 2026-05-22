//! Contains all the UI

use super::board;
use super::{State, UiScreen};
use eframe::egui::{self, Frame, ScrollArea};
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
                    self.file_ui(ui);
                }
                UiScreen::Board => {
                    // Dropdown box to select connection
                    egui::ComboBox::from_label("Select connection type")
                        .selected_text(format!("{:?}", self.connection_ui))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.connection_ui,
                                board::Connections::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.connection_ui,
                                board::Connections::M7CLMidi,
                                "M7CL over MIDI",
                            );
                        });
                    // Button to activate selected connection
                    if self.connection_ui != self.connection_ui_prev {
                        let response = ui.button("Activate selected connection");
                        if response.clicked() {
                            self.connection_ui_prev = self.connection_ui.clone();
                            self.connection = self.connection_ui.clone().construct();
                        }
                    }
                    ui.add_space(10.0);
                    // Ui specific to the connection
                    self.connection.ui(ui);
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
        const ROW_HEIGHT: f32 = 30.0;

        let num_dcas = self.num_dcas();
        TableBuilder::new(ui)
            // All columns must be "pre-allocated"
            .columns(Column::auto(), (num_dcas + 1).into())
            .auto_shrink(egui::Vec2b::FALSE)
            .striped(true)
            .header(HEADER_HEIGHT, |mut header| {
                header.col(|ui| {
                    ui.heading("Cue");
                });
                for i in 0..num_dcas {
                    header.col(|ui| {
                        ui.heading(format!("DCA {}", i + 1));
                    });
                }
            })
            .body(|mut body| {
                body.rows(ROW_HEIGHT, self.cues().len(), |mut row| {
                    let i = row.index();
                    let cue = &self.cues()[i];
                    row.col(|ui| {
                        ui.label(cue.name());
                    });
                    for (i, dca) in cue.dcas().iter().take(num_dcas.into()).enumerate() {
                        row.col(|ui| {
                            let dca_name = dca.name(self.channel_names());
                            if dca_name != "" {
                                ui.label(format!("{}", dca.name(self.channel_names())));
                            } else {
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{}", i+1))
                                            .weak()
                                            .italics()
                                    );
                                });
                            }
                        });
                    }
                })
            });
    }
    /// Draw the UI of the file screen
    fn file_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("save").clicked() {
                self.brick_save();
            }
            if ui.button("load").clicked() {
                self.brick_load();
            }
        });
    }
}
