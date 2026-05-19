//! Contains implementation for connecting to the Yamaha M7CL over MIDI

use crate::app::Cue;
use eframe::egui::{self, Ui, RichText, Color32};
use midir::{self, MidiOutput, MidiOutputConnection, MidiOutputPort};

/// The default value of the `no_touchy` setting
const DEFAULT_NO_TOUCHY: bool = false;
/// The default value of the `num_channels_controlled` setting
const DEFAULT_NUM_CH_CONTROL: u8 = 32;
/// Name of the application in MIDI
const MIDI_CLIENT_NAME: &str = "miq-v2";
/// Certain MIDI implementations have a name for the connection
const MIDI_CONNECTION_NAME: &str = "miq-v2 connection to Yamaha M7CL";
/// The MIDI channel index (ch1 = 0) to send on. MUST BE LESS THAN 16
const MIDI_CHANNEL_IND: u8 = 0;

/// A connection over MIDI to an M7CL.
pub struct M7CLMidi {
    conn: ConnectionState,
    /// If this is true, logic will assume that no one else touches the board appart from this.
    /// This lets it send fewer commands and is a hack until we implement recieving state update
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

impl super::Connectable for M7CLMidi {
    fn new() -> Self {
        let midi = MidiOutput::new(MIDI_CLIENT_NAME);
        match midi {
            Ok(midi) => {
                let ports = midi.ports();
                M7CLMidi {
                    conn: ConnectionState::YesMidiNoConnection(midi, ports, None),
                    no_touchy: DEFAULT_NO_TOUCHY,
                    board_state: None,
                    num_channels_controlled: DEFAULT_NUM_CH_CONTROL,
                }
            }
            Err(init_error) => M7CLMidi {
                conn: ConnectionState::NoMidi(init_error),
                no_touchy: DEFAULT_NO_TOUCHY,
                board_state: None,
                num_channels_controlled: DEFAULT_NUM_CH_CONTROL,
            },
        }
    }
    fn num_channels(&self) -> u8 {
        self.num_channels_controlled
    }
    fn num_dcas(&self) -> u8 {
        8
    }
    fn fire_cue(&mut self, cue: &Cue) {
        if self.no_touchy && matches!(self.board_state, Some(_)) {
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
    fn ui(&mut self, ui: &mut Ui) {
        ui.checkbox(&mut self.no_touchy, "No touchy: Assume that nothing other than this application will change mutes or assign DCAs or such");
        ui.label(format!("Num channels controlled: {}", self.num_channels_controlled));
        ui.add_space(10.0);
        match self.conn {
            ConnectionState::NoMidi(init_error) => {
                ui.label(RichText::new(format!("Failed to initialize MIDI: {}", init_error)).color(Color32::RED));
                // Retry button
                let response = ui.button("Retry");
                if response.clicked() { self.try_init_midi(); }
            }
            ConnectionState::YesMidiNoConnection(..) => {
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
                if ui.button("Reload ports").clicked() { self.update_ports_list(); }
            }
            ConnectionState::Connected(_) => {
                ui.label(RichText::new("Connected").color(Color32::GREEN));
                if ui.button("Disconnect").clicked() { self.disconnect(); }
            }
        };
    }
}

/// For internal use
impl M7CLMidi {
    /// Fire the provided cue, taking into account the differences between it and `prev_cue` and
    /// updates cached board state
    fn fire_cue_diff(&mut self, cue: &Cue, prev_cue: &Cue) {
        let prev_bitset = Self::channel_dca_bitset(self, prev_cue);
        let next_bitset = Self::channel_dca_bitset(self, cue);
        for (ch_ind, (prev, next)) in prev_bitset.iter().zip(next_bitset).enumerate() {
            if *prev == 0 && next == 0 {
                // Channel was and is off, do nothing
            } else if *prev == 0 && next != 0 {
                // Channel was off and is now on
                for dca_ind in 0..8 {
                    let assigned = (next >> dca_ind) & 0b0000_0001 == 0b0000_0001;
                    if assigned {
                        self.send_ch_dca(ch_ind as u8, dca_ind, true);
                    }
                }
                self.send_ch_on(ch_ind as u8, true);
            } else if *prev != 0 && next == 0 {
                // Channel was on and is now off
                self.send_ch_on(ch_ind as u8, false);
                for dca_ind in 0..8 {
                    let was_assigned = (prev >> dca_ind) & 0b0000_0001 == 0b0000_0001;
                    if was_assigned {
                        self.send_ch_dca(ch_ind as u8, dca_ind, false);
                    }
                }
            } else {
                // Channel was on and is still on
                for dca_ind in 0..8 {
                    let is_assigned = (next >> dca_ind) & 0b0000_0001 == 0b0000_0001;
                    let was_assigned = (prev >> dca_ind) & 0b0000_0001 == 0b0000_0001;
                    if !was_assigned && is_assigned {
                        self.send_ch_dca(ch_ind as u8, dca_ind, true);
                    } else if was_assigned && !is_assigned {
                        self.send_ch_dca(ch_ind as u8, dca_ind, false);
                    }
                }
            }
        }
        self.update_board_state_with_fired(cue)
    }
    /// Fire the provided cue in full and updates cached board state
    fn fire_full_cue(&mut self, cue: &Cue) {
        let channel_dcas = Self::channel_dca_bitset(self, cue);

        // Send appropriate messages per channel
        for (ch_ind, channel) in channel_dcas.iter().enumerate() {
            for dca_ind in 0..8 {
                let assigned = (channel >> dca_ind) & 0b0000_0001 == 0b0000_0001;
                self.send_ch_dca(ch_ind as u8, dca_ind, assigned);
            }
            if *channel > 0 {
                self.send_ch_on(ch_ind as u8, true)
            } else {
                self.send_ch_on(ch_ind as u8, false)
            }
        }

        self.update_board_state_with_fired(cue);
    }
    /// Returns a `Vec` with one entry per controlled channel. The entry is a `u8` bitmap of which
    /// DCAs the channel is assigned to. Note that this board has 8 DCAs.
    fn channel_dca_bitset(&self, cue: &Cue) -> Vec<u8> {
        let mut channel_dcas: Vec<u8> = (0..self.num_channels_controlled).map(|_| 0).collect();

        // Switch from channels of DCA to DCAs of channel
        for (dca_ind, dca) in cue.dcas().iter().enumerate() {
            for channel in dca.assigned() {
                if channel.index() as usize > channel_dcas.len() - 1 {
                    continue;
                }
                channel_dcas[channel.index() as usize] |= 0b0000_0001 << dca_ind;
            }
        }
        channel_dcas
    }
    /// Update the cache of the board state assuming this cue has just been fired
    fn update_board_state_with_fired(&mut self, cue: &Cue) {
        if let Some(cue_old) = &self.board_state {
            self.board_state = Some(cue_old.superimpose(cue))
        } else {
            self.board_state = Some(cue.clone())
        }
    }
    /// Send the messages to turn on/off the given channel
    fn send_ch_on(&mut self, ch_ind: u8, on: bool) {
        let val = if on {
            0b1111_1111_1111_1111
        } else {
            0b0000_0000_0000_0000
        };
        self.send_nrpn(0x05b6 + (ch_ind as u16), val);
    }
    /// Send the messages to assign/unassign the given channel from the given dca
    fn send_ch_dca(&mut self, ch_ind: u8, dca_ind: u8, assigned: bool) {
        let data = if assigned { 0x01 } else { 0x00 };
        self.send_prm_sysex(
            0x003f,
            dca_ind as u16,
            ch_ind as u16,
            [0x00, 0x00, 0x00, 0x00, data],
        );
    }
    /// Send the sequence of midi messages which corresponds to the given NRPN control change
    /// Note: takes normal, not midi, bytes
    fn send_nrpn(&mut self, param: u16, val: u16) {
        let (param_msb, param_lsb) = Self::two_byte_midi_pack(param);
        let (val_msb, val_lsb) = Self::two_byte_midi_pack(val);
        self.send(&[
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Parameter MSB
            0x63,
            // NRPN Parameter MSB value
            param_msb,
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Parameter LSB
            0x62,
            // NRPN Parameter LSB value
            param_lsb,
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Data MSB
            0x06,
            // NRPN Data MSB value
            val_msb,
            // Control change + channel
            0b1011_0000 | MIDI_CHANNEL_IND,
            // NRPN Data LSB
            0x26,
            // NRPN Data LSB value
            val_lsb,
        ])
    }
    /// Send the midi sysex message to change parameter as given
    /// Note: takes normal, not midi, bytes
    fn send_prm_sysex(&mut self, elem: u16, ind: u16, cc: u16, dd: [u8; 5]) {
        let (e1, e2) = Self::two_byte_midi_pack(elem);
        let (i1, i2) = Self::two_byte_midi_pack(ind);
        let (c1, c2) = Self::two_byte_midi_pack(cc);
        self.send(&[
            // Sysex
            0xf0,
            // Manufacturer id (Yamaha)
            0x43,
            // Sub status + channel
            0b0001_0000 | MIDI_CHANNEL_IND,
            // Group id (Yamaha: digital mixer)
            0x3e,
            // Model id (M7CL)
            0x11,
            // Data category (parameters, not dump or lib or meter or whatever)
            0x01,
            // Element
            e1,
            e2,
            // Index
            i1,
            i2,
            // Channel
            c1,
            c2,
            // Data
            ((dd[0] << 4) | (dd[1] >> 4)) & 0b0111_1111,
            ((dd[1] << 3) | (dd[2] >> 5)) & 0b0111_1111,
            ((dd[2] << 2) | (dd[3] >> 6)) & 0b0111_1111,
            ((dd[3] << 1) | (dd[4] >> 7)) & 0b0111_1111,
            dd[4] & 0b0111_1111,
            // End sysex
            0xf7,
        ]);
    }
    /// Takes two normal bytes and packs them into two midi bytes. Note that this is lossy as midi
    /// bytes are 7 bits.
    fn two_byte_midi_pack(input: u16) -> (u8, u8) {
        (
            ((input >> 7) & 0b0111_1111) as u8,
            (input & 0b0111_1111) as u8,
        )
    }
    /// Send the provided midi message
    /// Note: takes 7-bit midi bytes, not normal bytes
    fn send(&mut self, message: &[u8]) {
        if let ConnectionState::Connected(conn) = &mut self.conn {
            let result = conn.send(message);
            // TODO: log if error
        } else {
            // TODO: log oopsie
        }
    }
}

/// For external use
impl M7CLMidi {
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
            // TODO: implement logging and log that someone did an oopsie
        }
    }
    /// For a connection with MIDI initialized but not connected, return the list of available
    /// ports
    pub fn ports(&self) -> Result<midir::MidiOutputPorts, ()> {
        if let ConnectionState::YesMidiNoConnection(_, ports, _) = &self.conn {
            Ok(ports.clone())
        } else {
            // TODO: logging and someone did an oopsie
            Err(())
        }
    }
    /// For a connection with MIDI initialized but not connected, update the list of MIDI output ports
    pub fn update_ports_list(&mut self) {
        if let ConnectionState::YesMidiNoConnection(midi, ports, _) = &mut self.conn {
            *ports = midi.ports();
        } else {
            // TODO: logging and say someone did an oopsie
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
                        let error = connection_error.kind();
                        let midi = connection_error.into_inner();
                        let ports = midi.ports();
                        ConnectionState::YesMidiNoConnection(midi, ports, Some(error))
                    }
                }
            } else {
                //TODO: logging and oopsie
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
                    // TODO: log very bad
                    conn
                }
            })
        } else {
            // TODO: log oopsie
        }
    }
}
