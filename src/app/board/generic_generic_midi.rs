//! Contains implementation for connecting a generic board over MIDI. Details for each board are
//! provided by a `board_messages::BoardEditAdaptor`.

use super::board_messages::BoardEditAdaptor;
use super::board_state_cache::BoardStateCache;
use crate::app::{ChannelNames, Cue};
use eframe::egui::{self, Color32, RichText, Ui};
use midir::{
    self, MidiInput, MidiInputConnection, MidiInputPort, MidiInputPorts, MidiOutput,
    MidiOutputConnection, MidiOutputPort, MidiOutputPorts,
};
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
    Adaptor: BoardEditAdaptor<Message = Vec<u8>> + Send + 'static,
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
    /// A message displayed in the ui which can be customized to provide user information specific
    /// to a particular board
    custom_message: String,

    // FIX: These should not be here! Put them in the input/output connection enums where they
    // belong.
    /// Cached list of output ports we can connect to
    ui_output_ports: Option<MidiOutputPorts>,
    /// Cached list of input ports we can connect to
    ui_input_ports: Option<MidiInputPorts>,
}

impl<Adaptor> GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<u8>> + Send,
{
    pub fn new(adaptor: Adaptor, custom_message: String) -> Self {
        GenericGenericMidi {
            conn: ConnectionState::new(),
            adaptor,
            num_channels_controlled: DEFAULT_NUM_CH_CONTROL,
            num_dcas_controlled: DEFAULT_NUM_DCA_CONTROL,
            custom_message,
            ui_output_ports: None,
            ui_input_ports: None,
        }
    }
    /// If we are connected, check for any board edits sent back from the callback and apply them to
    /// the cache if present
    fn update_board_state_cache(&mut self) {
        if let ConnectionState::Connected {
            input: _,
            output: _,
            callback_receiver,
            board_state_cache,
        } = &mut self.conn
        {
            while let Ok(midi_message) = callback_receiver.try_recv() {
                // TODO: Again, this is inefficient and we should not have to allocate like this
                self.adaptor
                    .recv_board_message(midi_message)
                    .map(|board_edit| {
                        board_edit.for_each(|edit| board_state_cache.apply_board_edit(&edit))
                    });
            }
        } else {
            log::warn!("Cannot update board state cache because we are not connected");
        }
    }
    /// This is the callback called by midir when we receive a MIDI message. It takes the message,
    /// tries to parse it into a `BoardEdit`, and sends any parsed messages back to the main thread.
    fn midi_input_callback(_timestamp: u64, message: &[u8], sender: &mut mpsc::Sender<Vec<u8>>) {
        // FIX: This is inefficient and we should not have to allocate two vecs for this. Figure out
        // how to improve the api so that this isn't necessary.
        match sender.send(Vec::from(message)) {
            // Edit sent successfully
            Ok(()) => (),
            Err(_) => log::error!(
                "Tried to send a MIDI message back from the callback but the receiver has hung up. Something is very wrong."
            ),
        }
    }
}

/// Implementation of the interface that allows this to be used as a connectable board
impl<Adaptor> super::Connectable for GenericGenericMidi<Adaptor>
where
    Adaptor: BoardEditAdaptor<Message = Vec<u8>> + Send,
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
    // Every frame, check for messages received from the callback
    fn heartbeat(&mut self) {
        if matches!(self.conn, ConnectionState::Connected { .. }) {
            self.update_board_state_cache();
        }
    }
    fn fire_cue(&mut self, cue: &Cue) {
        if matches!(self.conn, ConnectionState::Connected { .. }) {
            self.update_board_state_cache();
        }
        if let ConnectionState::Connected {
            input: _,
            output,
            callback_receiver: _,
            board_state_cache,
        } = &mut self.conn
        {
            // FIXME: We are currently not sending DCA names. Fix the `Connectable` trait so that
            // this method gets &ChannelNames.
            let edits = board_state_cache.cue_diff_and_apply(
                cue,
                None,
                self.num_channels_controlled,
                self.num_dcas_controlled,
            );
            for edit in edits {
                match self.adaptor.send_board_edit(edit) {
                    Ok(messages) => {
                        for message in messages {
                            if let Err(err) = output.send(&message) {
                                log::error!("Failed to send MIDI message: {err}");
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to convert `BoardEdit` to MIDI message: {:?}", err)
                    }
                }
            }
        } else {
            log::warn!("Tried to fire cue but we are not connected")
        }
    }
    fn fire_channel_names(&mut self, names: &ChannelNames) {
        if let ConnectionState::Connected { output, .. } = &mut self.conn {
            for (ch_ind, name) in names.iterator() {
                match self.adaptor.send_board_edit(super::BoardEdit::ChannelName(
                    super::Channel::from_index(*ch_ind),
                    name.to_string(),
                )) {
                    Ok(messages) => messages.for_each(|msg| {
                        if let Err(err) = output.send(&msg) {
                            log::error!("Failed to send MIDI message: {err}");
                        }
                    }),
                    Err(err) => log::warn!(
                        "Tried to send channel name but encountered error adapting `BoardEdit` to correct format: {:?}",
                        err
                    ),
                }
            }
        } else {
            log::error!("`fire_channel_names` called but we are not connected");
        }
    }
    fn ui(&mut self, ui: &mut Ui) {
        ui.label(&self.custom_message);
        ui.add_space(10.0);
        ui.label(format!(
            "Num channels controlled: {}",
            self.num_channels_controlled
        ));
        ui.label(format!("Num DCAs controlled: {}", self.num_dcas_controlled));
        ui.add_space(10.0);
        match &mut self.conn {
            ConnectionState::NotConnected(output, input) => {
                ui.label("Not connected");
                match output {
                    OutputConnectionState::NoMidi(err) => {
                        ui.label(
                            RichText::new(format!("Failed to initialize MIDI output: {err}"))
                                .color(Color32::RED),
                        );
                        if ui.button("Initialize MIDI output").clicked() {
                            output.try_init_midi();
                        }
                    }
                    OutputConnectionState::YesMidiNoConnection(_, maybe_conn_err) => {
                        if let Some(err) = maybe_conn_err {
                            ui.label(
                                RichText::new(format!("Failed to connect: {err}"))
                                    .color(Color32::RED),
                            );
                        }
                        egui::ComboBox::from_label("Select MIDI output corresponding to board")
                            .selected_text("Ports")
                            .show_ui(ui, |ui| {
                                if let Some(ports) = &self.ui_output_ports {
                                    let mut responses = Vec::new();
                                    // Draw buttons for each port
                                    for port in ports {
                                        responses.push(ui.button(port.id()));
                                    }
                                    // Check if a button was pressed
                                    for (i, response) in responses.iter().enumerate() {
                                        if response.clicked() {
                                            // FIX: Use `take_or_recover` instead which gives better
                                            // panic behavior. Apply this to all uses of `take_mut::take`
                                            take_mut::take(output, |output| {
                                                output.try_connect(&ports[i])
                                            });
                                            break;
                                        }
                                    }
                                } else {
                                    ui.label("No known ports");
                                    ui.label("Try refreshing ports list");
                                }
                            });
                        if ui.button("Refresh ports list").clicked() {
                            match output.get_ports() {
                                Ok(ports) => self.ui_output_ports = Some(ports),
                                Err(_err) => log::error!("Failed to get MIDI output ports"),
                            }
                        }
                    }
                    OutputConnectionState::Connected(_) => {
                        ui.label(
                            RichText::new(format!("MIDI output connected")).color(Color32::GREEN),
                        );
                        if ui.button("Disconnect output").clicked() {
                            take_mut::take(output, |output| output.disconnect());
                        }
                    }
                }
                ui.add_space(10.0);
                match input {
                    InputConnectionState::NoMidi(err) => {
                        ui.label(
                            RichText::new(format!("Failed to initialize MIDI input: {err}"))
                                .color(Color32::RED),
                        );
                        if ui.button("Initialize MIDI input").clicked() {
                            take_mut::take(input, |input| input.try_init_midi());
                        }
                    }
                    InputConnectionState::YesMidiNoConnection(_, maybe_conn_err) => {
                        if let Some(err) = maybe_conn_err {
                            ui.label(
                                RichText::new(format!("Failed to connect: {err}"))
                                    .color(Color32::RED),
                            );
                        }
                        egui::ComboBox::from_label("Select MIDI input corresponding to board")
                            .selected_text("Ports")
                            .show_ui(ui, |ui| {
                                if let Some(ports) = &self.ui_input_ports {
                                    let mut responses = Vec::new();
                                    // Draw buttons for each port
                                    for port in ports {
                                        responses.push(ui.button(port.id()));
                                    }
                                    // Check if a button was pressed
                                    for (i, response) in responses.iter().enumerate() {
                                        if response.clicked() {
                                            take_mut::take(input, |input| {
                                                input.try_connect::<Adaptor>(&ports[i])
                                            });
                                            break;
                                        }
                                    }
                                } else {
                                    ui.label("No known ports");
                                    ui.label("Try refreshing ports list");
                                }
                            });
                        if ui.button("Refresh ports list").clicked() {
                            match input.get_ports() {
                                Ok(ports) => self.ui_input_ports = Some(ports),
                                Err(_err) => log::error!("Failed to get MIDI input ports"),
                            }
                        }
                    }
                    InputConnectionState::Connected(..) => {
                        ui.label(
                            RichText::new(format!("MIDI input connected")).color(Color32::GREEN),
                        );
                        if ui.button("Disconnect input").clicked() {
                            take_mut::take(input, |input| input.disconnect());
                        }
                    }
                }
                // If both input and output are connected, we are fully connected and should go to
                // the `Connected` state
                if matches!(output, OutputConnectionState::Connected(..))
                    && matches!(input, InputConnectionState::Connected(..))
                {
                    take_mut::take(&mut self.conn, |conn: ConnectionState| conn.connect());
                }
            }
            ConnectionState::Connected { .. } => {
                ui.label(RichText::new("Connected").color(Color32::GREEN));
                if ui.button("Disconnect").clicked() {
                    self.conn.disconnect();
                }
            }
        };
    }
}

/// State machine representing the state of the connection to the board
// TODO: Make all three of these a typestate
enum ConnectionState {
    NotConnected(OutputConnectionState, InputConnectionState),
    Connected {
        output: MidiOutputConnection,
        input: MidiInputConnection<mpsc::Sender<Vec<u8>>>,
        callback_receiver: mpsc::Receiver<Vec<u8>>,
        board_state_cache: BoardStateCache,
    },
}
impl ConnectionState {
    /// Create new connection objects. Takes the adaptor used by the input connection
    fn new() -> Self {
        ConnectionState::NotConnected(OutputConnectionState::new(), InputConnectionState::new())
    }
    /// For a connection which is "disconnected" but both input and output connections are
    /// connected, transition to the `Connected` state.
    fn connect(self) -> Self {
        match self {
            ConnectionState::NotConnected(output_conn, input_conn) => {
                match (output_conn, input_conn) {
                    (
                        OutputConnectionState::Connected(output),
                        InputConnectionState::Connected(
                            input,
                            callback_receiver,
                            board_state_cache,
                        ),
                    ) => ConnectionState::Connected {
                        output,
                        input,
                        callback_receiver,
                        board_state_cache,
                    },
                    (output_conn, input_conn) => {
                        log::warn!(
                            "Tried to transition to connected state but we are not ready to connect"
                        );
                        ConnectionState::NotConnected(output_conn, input_conn)
                    }
                }
            }
            _ => {
                log::warn!("Tried to transition to connected state but we are already connected");
                self
            }
        }
    }
    /// For a connection which is connected, disconnect.
    // TODO: Have the method take self instead of this take_mut nonsense
    fn disconnect(&mut self) {
        take_mut::take(self, |conn_state| match conn_state {
            ConnectionState::Connected { output, input, .. } => ConnectionState::NotConnected(
                OutputConnectionState::disconnect_conn(output),
                InputConnectionState::disconnect_conn(input),
            ),
            ConnectionState::NotConnected(..) => {
                log::warn!("Tried to disconnect a connection which is not connected");
                conn_state
            }
        });
    }
}
/// State machine representing the state of the outgoing connection to the board. Some of the
/// variants have adaptors in them because that is where the input adaptor is stored while it's not
/// in the connection.
enum OutputConnectionState {
    NoMidi(midir::InitError),
    YesMidiNoConnection(MidiOutput, Option<midir::ConnectErrorKind>),
    Connected(MidiOutputConnection),
}
impl OutputConnectionState {
    /// Create a new output connection object, attempting to initialize MIDI
    fn new() -> Self {
        match MidiOutput::new(MIDI_CLIENT_NAME) {
            Ok(midi_output) => {
                log::info!("Successfully initialized MIDI (output)");
                OutputConnectionState::YesMidiNoConnection(midi_output, None)
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
            OutputConnectionState::NoMidi(_) => match MidiOutput::new(MIDI_CLIENT_NAME) {
                Ok(midi_output) => {
                    log::info!("Successfully initialized MIDI (output)");
                    *self = OutputConnectionState::YesMidiNoConnection(midi_output, None);
                }
                Err(init_err) => {
                    log::warn!("Failed to initialize MIDI (output): {init_err}");
                    *self = OutputConnectionState::NoMidi(init_err);
                }
            },
            _ => {
                log::warn!(
                    "Tried to initialize MIDI on an output connection which already has MIDI"
                );
            }
        }
    }
    /// Get the ports available to connect to in a `YesMidiNoConnection`
    fn get_ports(&self) -> Result<MidiOutputPorts, ()> {
        match self {
            OutputConnectionState::YesMidiNoConnection(midi_output, _) => Ok(midi_output.ports()),
            _ => {
                log::warn!(
                    "Tried to get ports from an output connection which is not in a state to connect to ports"
                );
                Err(())
            }
        }
    }
    /// Try to connect to the given port
    fn try_connect(self, port: &MidiOutputPort) -> Self {
        if let OutputConnectionState::YesMidiNoConnection(midi_output, _) = self {
            match midi_output.connect(port, MIDI_CONNECTION_NAME) {
                Ok(connection) => {
                    log::info!("Successfully connected to MIDI output");
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
    /// Helper function to retrieve the MIDI object and reset to a ready-to-connect state
    fn disconnect_conn(conn: MidiOutputConnection) -> Self {
        Self::disconnect(OutputConnectionState::Connected(conn))
    }
}
/// State machine representing the state of the incoming connection from the board. This also stores
/// the input adaptor while it isn't being used.
enum InputConnectionState {
    NoMidi(midir::InitError),
    YesMidiNoConnection(MidiInput, Option<midir::ConnectErrorKind>),
    Connected(
        MidiInputConnection<mpsc::Sender<Vec<u8>>>,
        mpsc::Receiver<Vec<u8>>,
        BoardStateCache,
    ),
}
impl InputConnectionState {
    /// Create a new input connection object, taking the adaptor to be used in the connection
    fn new() -> Self {
        match MidiInput::new(MIDI_CLIENT_NAME) {
            Ok(midi_input) => {
                log::info!("Successfully initialized MIDI input");
                InputConnectionState::YesMidiNoConnection(midi_input, None)
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
            InputConnectionState::NoMidi(_) => match MidiInput::new(MIDI_CLIENT_NAME) {
                Ok(midi_input) => {
                    log::info!("Successfully initialized MIDI input");
                    InputConnectionState::YesMidiNoConnection(midi_input, None)
                }
                Err(init_err) => {
                    log::warn!("Failed to initialize MIDI output: {init_err}");
                    InputConnectionState::NoMidi(init_err)
                }
            },
            _ => {
                log::warn!(
                    "Tried to initialize MIDI on an output connection which already has MIDI"
                );
                self
            }
        }
    }
    /// Get the ports available to connect to in a `YesMidiNoConnection`
    fn get_ports(&self) -> Result<MidiInputPorts, ()> {
        match self {
            InputConnectionState::YesMidiNoConnection(midi_input, _) => Ok(midi_input.ports()),
            _ => {
                log::warn!(
                    "Tried to get ports from an input connection which is not in a state to connect to ports"
                );
                Err(())
            }
        }
    }
    /// Try to connect to the given port
    fn try_connect<Adaptor: BoardEditAdaptor<Message = Vec<u8>> + Send + 'static>(
        self,
        port: &MidiInputPort,
    ) -> Self {
        if let InputConnectionState::YesMidiNoConnection(midi_input, _) = self {
            let (tx, rx) = mpsc::channel();
            match midi_input.connect(
                port,
                MIDI_CONNECTION_NAME,
                GenericGenericMidi::<Adaptor>::midi_input_callback,
                tx,
            ) {
                Ok(connection) => {
                    log::info!("Successfully connected to MIDI input");
                    InputConnectionState::Connected(connection, rx, BoardStateCache::default())
                }
                Err(connect_error) => {
                    let err = connect_error.kind();
                    log::warn!("MIDI input failed to connect: {err}");
                    InputConnectionState::YesMidiNoConnection(connect_error.into_inner(), Some(err))
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
    /// Helper to retrieve the MIDI object from a connection we want to disconnect and return to a ready-to-connect state
    fn disconnect_conn(conn: MidiInputConnection<mpsc::Sender<Vec<u8>>>) -> Self {
        let (midi_input, _tx) = conn.close();
        InputConnectionState::YesMidiNoConnection(midi_input, None)
    }
}
