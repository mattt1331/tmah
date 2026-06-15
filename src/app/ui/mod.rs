//! Contains all the UI

use super::board;
use super::{Channel, CueNumber, State};
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

/// Whether and what editing action is the ui doing in the cues screen. For instance, the edit DCA
/// assignment popup or changing a cue description.
#[derive(Default)]
pub enum CuesEditAction {
    #[default]
    None,
    EditChannelNames,
    EditDcaAssign {
        cue_ind: usize,
        dca_ind: usize,
        original_dca_name: Option<String>,
    },
    EditCueDesc {
        cue_ind: usize,
    },
    RenumberCue {
        cue_ind: usize,
        input_text: String,
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
    // TODO: Extract stuff into separate functions
    /// Draw the UI of the area that shows the cues and DCAs
    fn cues_ui(&mut self, ui: &mut egui::Ui) {
        self.cues_ui_popup(ui);

        ui.horizontal(|ui| {
            if ui.button("Add cue at bot.").clicked() {
                let mut new_bottom_num = self.cues().keys().last().cloned().unwrap_or_default();
                new_bottom_num.increment_lowest();
                self.add_cue(super::Cue::default(), new_bottom_num, false);
            }
            if ui.button("Undo").clicked() {
                self.cues_do_undo();
            }
            if ui.button("Channel names").clicked() {
                self.do_cues_edit_action(CuesEditAction::EditChannelNames);
            }
        });
        ui.add_space(10.0);

        // Draw the grid with cues and dcas
        let double_clicked_cell = self.cues_table(ui);

        // If a DCA was double clicked, edit its assignment
        if let Some((i, j)) = double_clicked_cell {
            self.do_cues_edit_action(CuesEditAction::EditDcaAssign {
                cue_ind: i,
                dca_ind: j,
                original_dca_name: self
                    .cues()
                    .values()
                    .nth(i)
                    .map(|cue| &cue.dcas[j])
                    .and_then(|dca| dca.name.clone()),
            });
        }

        // If del key pressed, delete selected cue
        if ui.ctx().input(|input| input.key_pressed(egui::Key::Delete))
            && let Some(index) = self.selected_cue()
            && let Some(cue_num) = self.cues().keys().nth(index)
        {
            let cue_num = cue_num.clone();
            self.delete_cue(&cue_num, false);
        }

        // Space to GO
        if ui.ctx().input(|input| input.key_pressed(egui::Key::Space))
            && matches!(self.cues_edit_action(), CuesEditAction::None)
        {
            self.fire_next_cue();
        }
    }
    /// Draw the cues table and stuff. Returns if and which dca assignment cell was double-clicked
    /// as `(cue_ind, dca_ind)`.
    fn cues_table(&mut self, ui: &mut egui::Ui) -> Option<(usize, usize)> {
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
                    let cue_response = row.col(|ui| {
                        if let CuesEditAction::RenumberCue {
                            cue_ind,
                            input_text,
                        } = self.cues_edit_action_mut()
                            && *cue_ind == i
                        {
                            let response = ui.text_edit_singleline(input_text);
                            // If finished editing cue number
                            if response.lost_focus() {
                                let input: Result<CueNumber, _> = CueNumber::parse(input_text);
                                let cue_ind = *cue_ind; // copying the value because borrow checker
                                if let Ok(new_cue_num) = input
                                    && let Some(current_cue_num) = self.cues().keys().nth(cue_ind)
                                {
                                    self.renumber_cue(
                                        (*current_cue_num).clone(),
                                        new_cue_num,
                                        false,
                                    );
                                }
                                self.end_cues_edit_action();
                            }
                            response.request_focus();
                        } else {
                            ui.horizontal_centered(|ui| {
                                let cue_num = self
                                    .cues()
                                    .keys()
                                    .nth(i)
                                    .map(|n| n.to_string())
                                    .unwrap_or("?".to_string());
                                ui.add(egui::Label::new(cue_num).selectable(false));
                            });
                        }
                    });
                    if cue_response.1.double_clicked() {
                        let cue_num = self
                            .cues()
                            .keys()
                            .nth(i)
                            .map(|n| n.to_string())
                            .unwrap_or("?".to_string());
                        self.do_cues_edit_action(CuesEditAction::RenumberCue {
                            cue_ind: i,
                            input_text: cue_num,
                        })
                    }
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
                                if let Some(cue) = self.cues_mut().values_mut().nth(cue_ind) {
                                    let response = ui.text_edit_singleline(cue.edit_name());
                                    if response.lost_focus() {
                                        self.end_cues_edit_action();
                                    }
                                    response.request_focus();
                                } else {
                                    log::warn!(
                                        "Unable to get cue at index {cue_ind} to edit description"
                                    );
                                }
                            } else {
                                let desc_text = if let Some(cue) = self.cues().values().nth(i) {
                                    cue.name()
                                } else {
                                    ""
                                };
                                ui.add(egui::Label::new(desc_text).selectable(false));
                            }
                        });
                    });
                    if desc_response.1.double_clicked() {
                        self.do_cues_edit_action(CuesEditAction::EditCueDesc { cue_ind: i })
                    }
                    // DCAs
                    if let Some(cue) = &self.cues().values().nth(i) {
                        for (j, dca) in cue.dcas().iter().take(num_dcas.into()).enumerate() {
                            let (_, response) = row.col(|ui| {
                                let dca_name = dca.name(self.channel_names());
                                if !dca_name.is_empty() {
                                    let layout = egui::Layout::top_down(egui::Align::Center)
                                        .with_main_justify(true)
                                        .with_cross_align(egui::Align::Center);
                                    ui.with_layout(layout, |ui| {
                                        ui.add(
                                            egui::Label::new(dca_name.to_string())
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
                    } else {
                        log::warn!("Could not find cue at index {i}");
                    }
                })
            });
        double_clicked_cell
    }
    /// Draw the popup, if any, in the cues screen
    fn cues_ui_popup(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        if !ctx.egui_wants_keyboard_input() && ctx.input(|input| input.key_down(egui::Key::Escape))
        {
            self.end_cues_edit_action();
        }
        match *self.cues_edit_action() {
            CuesEditAction::None => (),
            CuesEditAction::EditChannelNames => {
                egui::Window::new("Edit Channel Names")
                    .title_bar(false)
                    .show(ui.ctx(), |ui| {
                        ui.heading("Edit channel names");
                        ui.add_space(10.0);
                        // Button to send channel names
                        if ui
                            .add_enabled(
                                self.connection().connected(),
                                egui::Button::new("Send channel names to board"),
                            )
                            .clicked()
                        {
                            self.fire_channel_names();
                        }
                        ui.add_space(10.0);
                        // Table of editable channel names
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
            CuesEditAction::EditDcaAssign { .. } => self.cues_ui_popup_dca_assign(ui),
            CuesEditAction::EditCueDesc { .. } | CuesEditAction::RenumberCue { .. } => (), // Not a popup
        }
    }
    /// Draw the _edit DCA assignment_ popup in the cues screen
    fn cues_ui_popup_dca_assign(&mut self, ui: &mut egui::Ui) {
        // Here, we copy the cue and also the indices to satisfy the borrow checker. At the bottom
        // of the function, we assign `copied_cue` back to the actual cue.
        let (mut copied_cue, cue_ind, _dca_ind) = if let CuesEditAction::EditDcaAssign {
            cue_ind,
            dca_ind,
            original_dca_name: _,
        } = self.cues_edit_action()
        {
            let cue = self.cues().values().nth(*cue_ind).cloned();
            if let Some(cue) = cue {
                (cue, *cue_ind, *dca_ind)
            } else {
                log::warn!("Unable to get cue we are editing");
                return;
            }
        } else {
            log::warn!("Edit dca assign popup called but we are not editing a dca");
            return;
        };
        if let CuesEditAction::EditDcaAssign {
            cue_ind,
            dca_ind,
            original_dca_name: _,
        } = self.cues_edit_action()
        {
            egui::Window::new("Edit DCA Assignments")
                .title_bar(false)
                .show(ui.ctx(), |ui| {
                    let dca_name = copied_cue.dcas()[*dca_ind].name(self.channel_names());
                    let dca_name = if !dca_name.is_empty() {
                        dca_name
                    } else {
                        (dca_ind + 1).to_string()
                    };
                    // Heading
                    ui.heading(format!("Cue {}: DCA {}", *cue_ind + 1, dca_name));
                    ui.add_space(10.0);
                    // DCA name edit
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        let dca_name = copied_cue.dcas_mut()[*dca_ind].edit_name();
                        if let Some(name) = dca_name {
                            ui.text_edit_singleline(name);
                            if name.is_empty() {
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
                    for ch in copied_cue.dcas()[*dca_ind].assigned() {
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
                                copied_cue.dcas_mut()[*dca_ind]
                                    .assign(Channel::from_index(ch_ind as u8))
                            } else {
                                copied_cue.dcas_mut()[*dca_ind]
                                    .unassign(Channel::from_index(ch_ind as u8))
                            }
                        }
                    }
                });
        } else {
            log::warn!("Edit dca assign popup called but we are not editing a dca");
        }
        let Some(actual_cue) = self.cues_mut().values_mut().nth(cue_ind) else {
            log::warn!("Unable to get cue at index {}", cue_ind);
            return;
        };
        *actual_cue = copied_cue;
    }
    /// Draw the UI of the file screen
    fn file_ui(&mut self, ui: &mut egui::Ui) {
        if self.file_is_idle() {
            ui.horizontal(|ui| {
                if ui.button("Save as").clicked() {
                    self.save_as();
                }
                if ui.button("Load").clicked() {
                    self.load();
                }
            });
        } else {
            if ui.button("Cancel").clicked() {
                self.cancel_file_dialog();
            }
        }
    }
}
