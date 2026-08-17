//! Contains implementation for connecting a generic board over MIDI. Details for each board are
//! provided by a `board_messages::BoardEditAdaptor`.

use super::data::BoardEdit;
use super::board_messages::BoardEditAdaptor;
use super::board_state_cache::BoardStateCache;
use crate::app::{ChannelNames, Cue};
use eframe::egui::{self, Color32, RichText, Ui};
use midir::{self, MidiOutput, MidiOutputConnection, MidiOutputPort, MidiInput, MidiInputConnection, MidiOutputPorts, MidiInputPorts, MidiInputPort};
use std::sync::mpsc;

/// The default value of the `num_channels_controlled` setting
const DEFAULT_NUM_CH_CONTROL: u8 = 32;
/// The default value of the `num_dcas_controlled` setting
const DEFAULT_NUM_DCA_CONTROL: u8 = 32;
/// Name of the application in MIDI
const MIDI_CLIENT_NAME: &str = "miq-v2";
/// Certain MIDI implementations have a name for the connection
const MIDI_CONNECTION_NAME: &str = "miq-v2 connection over MIDI";

/// A connection over MIDI to some board.
pub struct GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>> + Send + 'static,
{
    /// State machine representing our connection to the board
    conn: ConnectionState,
    /// Something which translates `BoardEdit`s into MIDI messages which can be sent over the
    /// connection.
    adaptor: Adaptor,
    /// We will only touch the first this many channels
    num_channels_controlled: u8,
    /// We will only touch the first this many channels
    num_dcas_controlled: u8,
}

impl<Adaptor> GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>> + Send,
{
    pub fn new(adaptor: Adaptor) -> Self {
        GenericGenericMidi {
            conn: ConnectionState::new(),
            adaptor,
            num_channels_controlled: DEFAULT_NUM_CH_CONTROL,
            num_dcas_controlled: DEFAULT_NUM_DCA_CONTROL,
        }
    }
    /// If we are connected, check for any board edits sent back from the callback and apply them to
    /// the cache if present
    fn update_board_state_cache(&mut self) {
        if let ConnectionState::Connected { input, output, callback_reciever, board_state_cache } = &mut self.conn {
            while let Ok(midi_message) = callback_reciever.try_recv() {
                // TODO: Again, this is inefficient and we should not have to allocate like this
                self.adaptor.recv_board_message(vec![midi_message])
                    .map(|board_edit| board_state_cache.apply_board_edit(&board_edit));
            }
        } else {
            log::warn!("Cannot update board state cache because we are not connected");
        }
    }
    /// This is the callback called by midir when we recieve a MIDI message. It takes the message,
    /// tries to parse it into a `BoardEdit`, and sends any parsed messages back to the main thread.
    fn midi_input_callback(_timestamp: u64, message: &[u8], sender: &mut mpsc::Sender<Vec<u8>>) {
        // FIX: This is inefficient and we should not have to allocate two vecs for this. Figure out
        // how to improve the api so that this isn't necessary.
        match sender.send(Vec::from(message)) {
                // Edit sent successfully
                Ok(()) => (),
                Err(_) => log::error!("Tried to send a MIDI message back from the callback but the reciever has hung up. Something is very wrong."),
            }
    }
}

/// Implementation of the interface that allows this to be used as a connectable board
impl<Adaptor> super::Connectable for GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>> + Send,
{
    fn num_channels(&self) -> u8 {
        self.num_channels_controlled
    }
    fn num_dcas(&self) -> u8 {
        self.num_dcas_controlled
    }
    fn connected(&self) -> bool {
        matches!(self.conn, ConnectionState::Connected { .. })
    }
    // Every frame, check for messages recieved from the callback
    fn heartbeat(&mut self) {
        if matches!(self.conn, ConnectionState::Connected { .. }) {
            self.update_board_state_cache();
        }
    }
    fn fire_cue(&mut self, cue: &Cue) {
        if matches!(self.conn, ConnectionState::Connected { .. }) {
            self.update_board_state_cache();
        }
        if let ConnectionState::Connected { input, output, callback_reciever, board_state_cache } = &mut self.conn {
            // FIXME: We are currently not sending DCA names. Fix the `Connectable` trait so that
            // this method gets &ChannelNames.
            let edits = board_state_cache.cue_diff_and_apply(cue, None, self.num_channels_controlled, self.num_dcas_controlled);
            for edit in edits {
                match self.adaptor.send_board_edit(edit) {
                    Ok(messages) => for message in messages {
                        output.send(&message);
                    }
                    Err(err) => log::error!("Failed to convert `BoardEdit` to MIDI message: {:?}", err),
                }
            }
        } else {
            log::warn!("Tried to fire cue but we are not connected")
        }
    }
    fn fire_channel_names(&mut self, names: &ChannelNames) {
        if let ConnectionState::Connected { output, .. } = &mut self.conn {
            names.iterator()
                .filter_map(|(ch_ind, name)| {
                    self.adaptor.send_board_edit(super::BoardEdit::ChannelName(
                        super::Channel::from_index(*ch_ind),
                        name.to_string(),
                    )).ok()
                })
                .for_each(|midi_messages| for message in midi_messages {
                    output.send(&message);
                });
        } else {
            log::error!("`fire_channel_names` called but we are not connected");
        }
    }
    //fn ui(&mut self, ui: &mut Ui) {
    //    ui.checkbox(&mut self.no_touchy, "No touchy: Assume that nothing other than this application will change mutes or assign DCAs or such");
    //    ui.label(format!(
    //        "Num channels controlled: {}",
    //        self.num_channels_controlled
    //    ));
    //    ui.add_space(10.0);
    //    match self.conn {
    //        ConnectionState::NoMidi(init_error) => {
    //            ui.label(
    //                RichText::new(format!("Failed to initialize MIDI: {}", init_error))
    //                    .color(Color32::RED),
    //            );
    //            // Retry button
    //            let response = ui.button("Retry");
    //            if response.clicked() {
    //                self.try_init_midi();
    //            }
    //        }
    //        ConnectionState::YesMidiNoConnection(_, _, conn_err) => {
    //            ui.label(RichText::new("MIDI initialized").color(Color32::GREEN));
    //            // Dropdown to select MIDI output
    //            egui::ComboBox::from_label("Select MIDI output corresponding to board")
    //                .selected_text("Ports")
    //                .show_ui(ui, |ui| {
    //                    if let Ok(ports) = self.ports() {
    //                        let mut responses = Vec::new();
    //                        for port in &ports {
    //                            responses.push(ui.button(port.id()));
    //                        }
    //                        for (i, response) in responses.iter().enumerate() {
    //                            if response.clicked() {
    //                                let port = ports[i].clone();
    //                                self.try_connect(port);
    //                            }
    //                        }
    //                    } else {
    //                        ui.label("Could not get ports");
    //                    }
    //                });
    //            // Reload ports list button
    //            if ui.button("Reload ports").clicked() {
    //                self.update_ports_list();
    //            }
    //            if let Some(conn_err) = conn_err {
    //                ui.label(
    //                    RichText::new(format!("Failed to connect: {}", conn_err))
    //                        .color(Color32::RED),
    //                );
    //            }
    //        }
    //        ConnectionState::Connected(_) => {
    //            ui.label(RichText::new("Connected").color(Color32::GREEN));
    //            if ui.button("Disconnect").clicked() {
    //                self.disconnect();
    //            }
    //        }
    //    };
    //}
}

/// State maching representing the state of the connection to the board
// TODO: Make all three of these a typestate
enum ConnectionState {
    NotConnected(OutputConnectionState, InputConnectionState),
    Connected {
        output: MidiOutputConnection,
        input: MidiInputConnection<mpsc::Sender<Vec<u8>>>,
        callback_reciever: mpsc::Receiver<Vec<u8>>,
        board_state_cache: BoardStateCache,
    },
}
impl ConnectionState {
    /// Create new connection objects. Takes the adaptor used by the input connection
    fn new() -> Self {
        ConnectionState::NotConnected(OutputConnectionState::new(), InputConnectionState::new())
    }
}
/// State machine representing the state of the outgoing connection to the board. Some of the
/// variants have adaptors in them because that is where the input adaptor is stored while it's not
/// in the connection.
enum OutputConnectionState {
    NoMidi(midir::InitError),
    YesMidiNoConnection(
        MidiOutput,
        Option<midir::ConnectErrorKind>
    ),
    Connected(MidiOutputConnection),
}
impl OutputConnectionState {
    /// Create a new output connection object, attempting to initialize MIDI
    fn new() -> Self {
        match MidiOutput::new(MIDI_CLIENT_NAME) {
            Ok(midi_output) => {
                log::info!("Succesfully initialized MIDI (output)");
                OutputConnectionState::YesMidiNoConnection(
                    midi_output,
                    None
                    )
            }
            Err(init_err) => {
                log::warn!("Failed to initialize MIDI (output): {init_err}");
                OutputConnectionState::NoMidi(init_err)
            }
        }
    }
    /// For a `NoMidi`, try to initialize MIDI
    fn try_init_midi(&mut self) {
        match self {
            OutputConnectionState::NoMidi(_) => {
                match MidiOutput::new(MIDI_CLIENT_NAME) {
                    Ok(midi_output) => {
                        log::info!("Succesfully initialized MIDI (output)");
                        *self = OutputConnectionState::YesMidiNoConnection(
                            midi_output,
                            None
                            );
                    }
                    Err(init_err) => {
                        log::warn!("Failed to initialize MIDI (output): {init_err}");
                        *self = OutputConnectionState::NoMidi(init_err);
                    }
                }
            }
            _ => {
                log::warn!("Tried to initialize MIDI on an output connection which already has MIDI");
            }
        }
    }
    /// Get the ports available to connect to in a `YesMidiNoConnection`
    fn get_ports(&self) -> Result<MidiOutputPorts, ()> {
        match self {
            OutputConnectionState::YesMidiNoConnection(midi_output, _) => Ok(midi_output.ports()),
            _ => {
                log::warn!("Tried to get ports from an output connection which is not in a state to connect to ports");
                Err(())
            }
        }
    }
    /// Try to connect to the given port
    fn try_connect(self, port: &MidiOutputPort) -> Self {
        if let OutputConnectionState::YesMidiNoConnection(midi_output, _) = self {
            match midi_output.connect(port, MIDI_CONNECTION_NAME) {
                Ok(connection) => {
                    log::info!("Succesfully connected to MIDI output");
                    OutputConnectionState::Connected(connection)
                }
                Err(connect_error) => {
                    let err = connect_error.kind();
                    log::warn!("MIDI output failed to connect: {err}");
                    OutputConnectionState::YesMidiNoConnection(
                        connect_error.into_inner(),
                        Some(err),
                        )
                }
            }
        } else {
            log::warn!("Tried to connect but this output connection is not in a state to connect");
            self
        }
    }
    /// Disconnect from the connected MIDI port
    fn disconnect(self) -> Self {
        if let OutputConnectionState::Connected(conn) = self {
            let midi_output = conn.close();
            OutputConnectionState::YesMidiNoConnection(midi_output, None)
        } else {
            log::warn!("Tried to disconnect an output connection which is not ocnnected");
            self
        }
    }
    /// Send the given message
    fn send(&mut self, message: &[u8]) {
        if let OutputConnectionState::Connected(output_con) = self {
            match output_con.send(message) {
                Ok(()) => (),
                Err(err) => log::warn!("Failed to send MIDI message: {err}"),
            }
        } else {
            log::warn!("Tried to send MIDI message but we are not connected")
        }
    }
}
/// State machine representing the state of the incoming connection from the board. This also stores
/// the input adaptor while it isn't being used.
enum InputConnectionState {
    NoMidi(midir::InitError),
    YesMidiNoConnection(
        MidiInput,
        Option<midir::ConnectErrorKind>,
    ),
    Connected(MidiInputConnection<mpsc::Sender<Vec<u8>>>, mpsc::Receiver<Vec<u8>>, BoardStateCache),
}
impl InputConnectionState {
    /// Create a new input connection object, taking the adaptor to be used in the connection
    fn new() -> Self {
        match MidiInput::new(MIDI_CLIENT_NAME) {
            Ok(midi_input) => {
                log::info!("Succesfully initialized MIDI input");
                InputConnectionState::YesMidiNoConnection(
                    midi_input,
                    None,
                    )
            }
            Err(init_err) => {
                log::warn!("Failed to initialize MIDI output: {init_err}");
                InputConnectionState::NoMidi(init_err)
            }
        }
    }
    /// For a `NoMidi`, try to initialize MIDI
    fn try_init_midi(self) -> Self {
        match self {
            InputConnectionState::NoMidi(_) => {
                match MidiInput::new(MIDI_CLIENT_NAME) {
                    Ok(midi_input) => {
                        log::info!("Succesfully initialized MIDI input");
                        InputConnectionState::YesMidiNoConnection(
                            midi_input,
                            None,
                            )
                    }
                    Err(init_err) => {
                        log::warn!("Failed to initialize MIDI output: {init_err}");
                        InputConnectionState::NoMidi(init_err)
                    }
                }
            }
            _ => {
                log::warn!("Tried to initialize MIDI on an output connection which already has MIDI");
                self
            }
        }
    }
    /// Get the ports available to connect to in a `YesMidiNoConnection`
    fn get_ports(&self) -> Result<MidiInputPorts, ()> {
        match self {
            InputConnectionState::YesMidiNoConnection(midi_input, _) => Ok(midi_input.ports()),
            _ => {
                log::warn!("Tried to get ports from an input connection which is not in a state to connect to ports");
                Err(())
            }
        }
    }
    /// Try to connect to the given port
    fn try_connect<Adaptor: BoardEditAdaptor<Message = Vec<Vec<u8>>> + Send + 'static>(self, port: &MidiInputPort) -> Self {
        if let InputConnectionState::YesMidiNoConnection(midi_input, _) = self {
            let (tx, rx) = mpsc::channel();
            match midi_input.connect(port, MIDI_CONNECTION_NAME, GenericGenericMidi::<Adaptor>::midi_input_callback, tx) {
                Ok(connection) => {
                    log::info!("Succesfully connected to MIDI input");
                    InputConnectionState::Connected(connection, rx, BoardStateCache::default())
                }
                Err(connect_error) => {
                    let err = connect_error.kind();
                    log::warn!("MIDI input failed to connect: {err}");
                    InputConnectionState::YesMidiNoConnection(
                        connect_error.into_inner(),
                        Some(err),
                        )
                }
            }
        } else {
            log::warn!("Tried to connect but this input connection is not in a state to connect");
            self
        }
    }
    /// Disconnect from the connected MIDI port
    fn disconnect(self) -> Self {
        if let InputConnectionState::Connected(conn, _receiver, _cache) = self {
            let (midi_input, _tx) = conn.close();
            InputConnectionState::YesMidiNoConnection(midi_input, None)
        } else {
            log::warn!("Tried to disconnect an input connection which is not ocnnected");
            self
        }
    }
}
