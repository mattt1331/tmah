//! Contains all the UI

use super::board;
use super::{Channel, State};
use eframe::egui;
use egui_extras::{Column, TableBuilder};

/// The different screens of the ui, like the cues, board connection, etc
#[derive(Default, PartialEq)]
pub enum UiScreen {
    #[default]
    Cues,
    File,
    Board,
}

/// Whether and what popup is active in the cues screen
#[derive(Default)]
pub enum CuesEditAction {
    #[default]
    None,
    EditChannelNames,
    EditDcaAssign {
        cue_ind: usize,
        dca_ind: usize,
    },
    EditCueDesc {
        cue_ind: usize,
    },
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
    // TODO: Extract stuff into seperate functions
    /// Draw the UI of the area that shows the cues and DCAs
    fn cues_ui(&mut self, ui: &mut egui::Ui) {
        self.cues_ui_popup(ui);

        ui.horizontal(|ui| {
            if ui.button("Channel names").clicked() {
                self.do_cues_edit_action(CuesEditAction::EditChannelNames);
            }
        });
        ui.add_space(10.0);

        const HEADER_HEIGHT: f32 = 20.0;
        const ROW_HEIGHT: f32 = 30.0;
        const DESC_WIDTH: f32 = 300.0;

        // Draw the ui
        let mut double_clicked_cell: Option<(usize, usize)> = None;
        let num_dcas = self.num_dcas();
        TableBuilder::new(ui)
            // All columns must be "pre-allocated"
            .column(Column::auto()) // Cue
            .column(Column::initial(DESC_WIDTH)) // Desc
            .columns(Column::auto(), num_dcas.into()) // DCAs
            .auto_shrink(egui::Vec2b::FALSE)
            .striped(true)
            .sense(egui::Sense::click())
            .header(HEADER_HEIGHT, |mut header| {
                header.col(|ui| {
                    ui.heading("Cue");
                });
                header.col(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Desc");
                    });
                });
                for i in 0..num_dcas {
                    header.col(|ui| {
                        ui.heading(format!("DCA {}", i + 1));
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_HEIGHT, self.cues().len(), |mut row| {
                    row.set_hovered(false); // Otherwise it does an ugly highlight when you mouse over

                    let i = row.index();

                    // Selected row
                    if let Some(sel_ind) = self.selected_cue()
                        && sel_ind == i
                    {
                        row.set_selected(true);
                    }

                    // Cue
                    row.col(|ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label(format!("{i}"));
                        });
                    });
                    // Desc
                    let desc_response = row.col(|ui| {
                        let layout = egui::Layout::left_to_right(egui::Align::Center)
                            .with_main_wrap(true)
                            .with_cross_justify(true);
                        ui.with_layout(layout, |ui| {
                            if let CuesEditAction::EditCueDesc { cue_ind } =
                                *self.cues_edit_action()
                                && cue_ind == i
                            {
                                let response =
                                    ui.text_edit_singleline(self.cues_mut()[cue_ind].edit_name());
                                if response.lost_focus() {
                                    self.clear_cues_edit_action();
                                }
                                response.request_focus();
                            } else {
                                ui.add(egui::Label::new(self.cues()[i].name()).selectable(false));
                            }
                        });
                    });
                    if desc_response.1.double_clicked() {
                        self.do_cues_edit_action(CuesEditAction::EditCueDesc { cue_ind: i })
                    }
                    // DCAs
                    let cue = &self.cues()[i];
                    for (j, dca) in cue.dcas().iter().take(num_dcas.into()).enumerate() {
                        let (_, response) = row.col(|ui| {
                            let dca_name = dca.name(self.channel_names());
                            if !dca_name.is_empty() {
                                let layout = egui::Layout::top_down(egui::Align::Center)
                                    .with_main_justify(true)
                                    .with_cross_align(egui::Align::Center);
                                ui.with_layout(layout, |ui| {
                                    ui.add(
                                        egui::Label::new(dca.name(self.channel_names()).to_string())
                                        .selectable(false),
                                    );
                                });
                            } else {
                                ui.centered_and_justified(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!("{}", j + 1))
                                                .weak()
                                                .italics(),
                                        )
                                        .selectable(false),
                                    );
                                });
                            }
                        });
                        if response.double_clicked() {
                            double_clicked_cell = Some((i, j));
                        }
                    }
                    // Select row if clicked
                    if row.response().clicked() {
                        self.set_selected_cue(Some(i));
                        self.fire_selected_cue();
                    }
                })
            });

        // If a DCA was double clicked, edit it's assignment
        if let Some((i, j)) = double_clicked_cell {
            self.do_cues_edit_action(CuesEditAction::EditDcaAssign {
                cue_ind: i,
                dca_ind: j,
            })
        }

        if ui.ctx().input(|input| input.key_pressed(egui::Key::Space))
            && matches!(self.cues_edit_action(), CuesEditAction::None)
        {
            self.fire_next_cue();
        }
    }
    /// Draw the popup, if any, in the cues screen
    fn cues_ui_popup(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        if !ctx.egui_wants_keyboard_input() && ctx.input(|input| input.key_down(egui::Key::Escape))
        {
            self.clear_cues_edit_action();
        }
        match *self.cues_edit_action() {
            CuesEditAction::None => (),
            CuesEditAction::EditChannelNames => {
                egui::Window::new("Edit Channel Names")
                    .title_bar(false)
                    .show(ui.ctx(), |ui| {
                        ui.heading("Edit channel names");
                        TableBuilder::new(ui)
                            .columns(Column::auto(), 2)
                            .striped(true)
                            .header(20.0, |mut header| {
                                header.col(|ui| {
                                    ui.heading("Channel");
                                });
                                header.col(|ui| {
                                    ui.heading("Name");
                                });
                            })
                            .body(|body| {
                                body.rows(20.0, self.num_channels().into(), |mut row| {
                                    let i = row.index();
                                    row.col(|ui| {
                                        ui.label(format!("Channel {}", i + 1));
                                    });
                                    row.col(|ui| {
                                        let Ok(i) = TryInto::<u8>::try_into(i) else {
                                            return;
                                        };
                                        ui.text_edit_singleline(
                                            self.channel_names_mut()
                                                .edit_name(&Channel::from_index(i)),
                                        );
                                    });
                                })
                            })
                    });
            }
            CuesEditAction::EditDcaAssign { cue_ind, dca_ind } => {
                egui::Window::new("Edit DCA Assignments")
                    .title_bar(false)
                    .show(ui.ctx(), |ui| {
                        let dca_name =
                            self.cues()[cue_ind].dcas()[dca_ind].name(self.channel_names());
                        let dca_name = if !dca_name.is_empty() {
                            dca_name
                        } else {
                            (dca_ind + 1).to_string()
                        };
                        // Heading
                        ui.heading(format!("Cue {}: DCA {}", cue_ind + 1, dca_name));
                        ui.add_space(10.0);
                        // DCA name edit
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            let dca_name = self.cues_mut()[cue_ind].dcas_mut()[dca_ind].edit_name();
                            if let Some(name) = dca_name {
                                ui.text_edit_singleline(name);
                                if !name.is_empty() {
                                    *dca_name = None;
                                }
                            } else {
                                let mut name = "".to_string();
                                ui.text_edit_singleline(&mut name);
                                if !name.is_empty() {
                                    *dca_name = Some(name);
                                }
                            }
                        });
                        ui.add_space(10.0);
                        // Channel assignments
                        let num_channels = self.num_channels();
                        let mut assigned: Vec<bool> = vec![false; num_channels.into()];
                        for ch in self.cues()[cue_ind].dcas()[dca_ind].assigned() {
                            assigned[ch.index() as usize] = true;
                        }
                        let assigned_pre = assigned.clone();
                        for i in 0..num_channels {
                            let ch_name = self
                                .channel_names()
                                .get_name(&Channel::from_index(i))
                                .unwrap_or_else(|| format!("Channel {}", i + 1));
                            let ch_name = if !ch_name.is_empty() {
                                ch_name
                            } else {
                                format!("Channel {}", i + 1)
                            };
                            ui.checkbox(&mut assigned[i as usize], ch_name);
                        }
                        for (ch_ind, (pre, post)) in
                            assigned_pre.iter().zip(assigned.iter()).enumerate()
                        {
                            if pre != post {
                                if *post {
                                    self.cues_mut()[cue_ind].dcas_mut()[dca_ind]
                                        .assign(Channel::from_index(ch_ind as u8))
                                } else {
                                    self.cues_mut()[cue_ind].dcas_mut()[dca_ind]
                                        .unassign(Channel::from_index(ch_ind as u8))
                                }
                            }
                        }
                    });
            }
            CuesEditAction::EditCueDesc { .. } => (), // Not a popup
        }
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
