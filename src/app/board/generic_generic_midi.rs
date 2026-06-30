//! Contains implementation for connecting a generic board over MIDI. Details for each board are
//! provided by a `board_messages::BoardEditAdaptor`.

use super::board_messages::BoardEditAdaptor;
use crate::app::{ChannelNames, Cue};
use eframe::egui::{self, Color32, RichText, Ui};
use midir::{self, MidiOutput, MidiOutputConnection, MidiOutputPort};

/// The default value of the `no_touchy` setting
const DEFAULT_NO_TOUCHY: bool = false;
/// The default value of the `num_channels_controlled` setting
const DEFAULT_NUM_CH_CONTROL: u8 = 32;
/// Name of the application in MIDI
const MIDI_CLIENT_NAME: &str = "miq-v2";
/// Certain MIDI implementations have a name for the connection
const MIDI_CONNECTION_NAME: &str = "miq-v2 connection over MIDI";

/// A connection over MIDI to some board.
pub struct GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor,
{
    conn: ConnectionState,
    /// Something which translates `BoardEdit`s into MIDI messages which can be sent over the
    /// connection.
    adaptor: Adaptor,
    /// If this is true, logic will assume that no one else touches the board apart from this.
    /// This lets it send fewer commands and is a hack until we implement receiving state update
    /// messages from the board.
    // TODO: implement receiving state update messages from the board
    no_touchy: bool,
    board_state: Option<Cue>,
    /// We will only touch the first this many channels
    num_channels_controlled: u8,
}

/// State machine representing the state of the connection to the board
enum ConnectionState {
    NoMidi(midir::InitError),
    YesMidiNoConnection(
        MidiOutput,
        midir::MidiOutputPorts,
        Option<midir::ConnectErrorKind>,
    ),
    Connected(MidiOutputConnection),
}

impl<Adaptor> GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>>,
{
    pub fn new(adaptor: Adaptor) -> Self {
        let midi = MidiOutput::new(MIDI_CLIENT_NAME);
        match midi {
            Ok(midi) => {
                let ports = midi.ports();
                GenericGenericMidi {
                    conn: ConnectionState::YesMidiNoConnection(midi, ports, None),
                    adaptor,
                    no_touchy: DEFAULT_NO_TOUCHY,
                    board_state: None,
                    num_channels_controlled: DEFAULT_NUM_CH_CONTROL,
                }
            }
            Err(init_error) => GenericGenericMidi {
                conn: ConnectionState::NoMidi(init_error),
                adaptor,
                no_touchy: DEFAULT_NO_TOUCHY,
                board_state: None,
                num_channels_controlled: DEFAULT_NUM_CH_CONTROL,
            },
        }
    }
}

impl<Adaptor> super::Connectable for GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>>,
{
    fn num_channels(&self) -> u8 {
        self.num_channels_controlled
    }
    fn num_dcas(&self) -> u8 {
        8
    }
    fn connected(&self) -> bool {
        matches!(self.conn, ConnectionState::Connected(_))
    }
    fn fire_cue(&mut self, cue: &Cue) {
        if self.no_touchy && self.board_state.is_some() {
            // no touchy assumes nothing else but this touches the board
            let Some(ref prev_cue) = self.board_state else {
                unreachable!("Matched `Some` above")
            };
            let prev_cue = prev_cue.clone();
            self.fire_cue_diff(cue, &prev_cue);
        } else {
            self.fire_full_cue(cue);
        }
    }
    fn fire_channel_names(&mut self, names: &ChannelNames) {
        if let ConnectionState::Connected(..) = self.conn {
            for (ch_ind, name) in names.iterator() {
                self.fire_board_edit(super::BoardEdit::ChannelName(
                    super::Channel::from_index(*ch_ind),
                    name.to_string(),
                ));
            }
        } else {
            log::error!("`fire_channel_names` called but we are not connected");
        }
    }
    fn ui(&mut self, ui: &mut Ui) {
        ui.checkbox(&mut self.no_touchy, "No touchy: Assume that nothing other than this application will change mutes or assign DCAs or such");
        ui.label(format!(
            "Num channels controlled: {}",
            self.num_channels_controlled
        ));
        ui.add_space(10.0);
        match self.conn {
            ConnectionState::NoMidi(init_error) => {
                ui.label(
                    RichText::new(format!("Failed to initialize MIDI: {}", init_error))
                        .color(Color32::RED),
                );
                // Retry button
                let response = ui.button("Retry");
                if response.clicked() {
                    self.try_init_midi();
                }
            }
            ConnectionState::YesMidiNoConnection(_, _, conn_err) => {
                ui.label(RichText::new("MIDI initialized").color(Color32::GREEN));
                // Dropdown to select MIDI output
                egui::ComboBox::from_label("Select MIDI output corresponding to M7CL")
                    .selected_text("Ports")
                    .show_ui(ui, |ui| {
                        if let Ok(ports) = self.ports() {
                            let mut responses = Vec::new();
                            for port in &ports {
                                responses.push(ui.button(port.id()));
                            }
                            for (i, response) in responses.iter().enumerate() {
                                if response.clicked() {
                                    let port = ports[i].clone();
                                    self.try_connect(port);
                                }
                            }
                        } else {
                            ui.label("Could not get ports");
                        }
                    });
                // Reload ports list button
                if ui.button("Reload ports").clicked() {
                    self.update_ports_list();
                }
                if let Some(conn_err) = conn_err {
                    ui.label(
                        RichText::new(format!("Failed to connect: {}", conn_err))
                            .color(Color32::RED),
                    );
                }
            }
            ConnectionState::Connected(_) => {
                ui.label(RichText::new("Connected").color(Color32::GREEN));
                if ui.button("Disconnect").clicked() {
                    self.disconnect();
                }
            }
        };
    }
}

/// For external use
/// (part of med level API)
impl<Adaptor> GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>>,
{
    /// For a connection which failed to initialize MIDI, retry initializing
    pub fn try_init_midi(&mut self) {
        if let ConnectionState::NoMidi(_) = self.conn {
            // Yes this is exactly like `Self::new`
            let midi = MidiOutput::new(MIDI_CLIENT_NAME);
            match midi {
                Ok(midi) => {
                    let ports = midi.ports();
                    self.conn = ConnectionState::YesMidiNoConnection(midi, ports, None)
                }
                Err(init_error) => self.conn = ConnectionState::NoMidi(init_error),
            }
        } else {
            log::warn!("`try_init_midi` called but we already initialized MIDI");
        }
    }
    /// For a connection with MIDI initialized but not connected, return the list of available
    /// ports
    pub fn ports(&self) -> Result<midir::MidiOutputPorts, ()> {
        if let ConnectionState::YesMidiNoConnection(_, ports, _) = &self.conn {
            Ok(ports.clone())
        } else {
            log::warn!("Get `ports` called but is not available in current state");
            Err(())
        }
    }
    /// For a connection with MIDI initialized but not connected, update the list of MIDI output ports
    pub fn update_ports_list(&mut self) {
        if let ConnectionState::YesMidiNoConnection(midi, ports, _) = &mut self.conn {
            *ports = midi.ports();
        } else {
            log::warn!("`update_ports_list` called but is not available in current state");
        }
    }
    /// For a connection with MIDI initialized but not connected, try to connect to the given port
    pub fn try_connect(&mut self, port: MidiOutputPort) {
        // Because connect needs ownership of midi, we need to do this maneuver to get `self.conn`
        // out from behind the reference. Note that we must return `conn` from the closure.
        take_mut::take(&mut self.conn, |conn| {
            if let ConnectionState::YesMidiNoConnection(midi, _, _) = conn {
                let connection = midi.connect(&port, MIDI_CONNECTION_NAME);
                match connection {
                    Ok(connection) => ConnectionState::Connected(connection),
                    Err(connection_error) => {
                        log::error!("Unable to connect to MIDI port: {connection_error}");
                        let error = connection_error.kind();
                        let midi = connection_error.into_inner();
                        let ports = midi.ports();
                        ConnectionState::YesMidiNoConnection(midi, ports, Some(error))
                    }
                }
            } else {
                log::warn!("`try_connect` called but is not available in current state");
                conn
            }
        });
    }
    /// For a connection which is connected, disconnect
    pub fn disconnect(&mut self) {
        if matches!(self.conn, ConnectionState::Connected(_)) {
            // See `try_connect` for why we do this
            take_mut::take(&mut self.conn, |conn| {
                if let ConnectionState::Connected(conn) = conn {
                    let midi = conn.close();
                    let ports = midi.ports();
                    ConnectionState::YesMidiNoConnection(midi, ports, None)
                } else {
                    log::error!(
                        "Should be in state `Connected` from `matches!` above but something has gone very wrong, failed to disconnect"
                    );
                    conn
                }
            })
        } else {
            log::warn!("`disconnect` called on a connection which is not connected");
        }
    }
}

/// For internal use
/// Old API (low + med level)
impl<Adaptor: BoardEditAdaptor> GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>>,
{
    /// Fire the provided cue, taking into account the differences between it and `prev_cue` and
    /// updates cached board state
    fn fire_cue_diff(&mut self, cue: &Cue, prev_cue: &Cue) {
        let diff = Cue::diff(prev_cue, cue);
        self.fire_diff(diff);
        self.update_board_state_with_fired(cue)
    }
    /// Fire the provided cue in full and updates cached board state
    fn fire_full_cue(&mut self, cue: &Cue) {
        // FIX: This is a hack. It works, but it's dumb. While decoupling this from app above and
        // from board edits below, fix this.
        let blank_cue = Cue::default();
        let diff = Cue::diff(&blank_cue, cue);
        self.fire_diff(diff);
        self.update_board_state_with_fired(cue);
    }
    /// Update the cache of the board state assuming this cue has just been fired
    fn update_board_state_with_fired(&mut self, cue: &Cue) {
        if let Some(cue_old) = &self.board_state {
            self.board_state = Some(cue_old.superimpose(cue))
        } else {
            self.board_state = Some(cue.clone())
        }
    }
}
/// Improved API
/// New API (low level)
impl<Adaptor> GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>>,
{
    // move into trait default impl
    fn fire_diff(&mut self, diff: Vec<super::BoardEdit>) {
        for edit in diff {
            self.fire_board_edit(edit);
        }
    }
    fn fire_board_edit(&mut self, edit: super::BoardEdit) {
        let message = self.adaptor.send_board_edit(edit);
        let message: Vec<Vec<u8>> = match message {
            Ok(msg) => msg,
            Err(err) => {
                log::error!("Failed to convert BoardEdit to midi message: {err:?}");
                return;
            }
        };
        for midi_message in message {
            self.send(&midi_message);
        }
    }
}
/// For internal use
impl<Adaptor: BoardEditAdaptor> GenericGenericMidi<Adaptor> {
    /// Send the provided midi message
    /// Note: takes 7-bit midi bytes, not normal bytes
    fn send(&mut self, message: &[u8]) {
        if let ConnectionState::Connected(conn) = &mut self.conn {
            let result = conn.send(message);
            match result {
                Ok(()) => log::info!("MIDI message sent"),
                Err(err) => log::error!("Failed to send MIDI message: {err}"),
            }
        } else {
            log::warn!("`send` MIDI message called but we are disconnected");
        }
    }
}
