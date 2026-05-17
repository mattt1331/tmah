//! Contains implementation for connecting to the Yamaha M7CL over MIDI

use midir::{self, MidiOutput, MidiOutputConnection, MidiOutputPort};

/// Name of the application in MIDI
const MIDI_CLIENT_NAME: &str = "miq-v2";
/// Certain MIDI implementations have a name for the connection
const MIDI_CONNECTION_NAME: &str = "miq-v2 connection to Yamaha M7CL";

/// A connection over MIDI to an M7CL.
/// Note: this is really a state machine
pub enum M7CLMidi {
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
                M7CLMidi::YesMidiNoConnection(midi, ports, None)
            }
            Err(init_error) => M7CLMidi::NoMidi(init_error),
        }
    }
}

impl M7CLMidi {
    /// For a connection which failed to initialize MIDI, retry initializing
    pub fn try_init_midi(&mut self) {
        if let M7CLMidi::NoMidi(_) = self {
            // Yes this is exactly like `Self::new`
            let midi = MidiOutput::new(MIDI_CLIENT_NAME);
            match midi {
                Ok(midi) => {
                    let ports = midi.ports();
                    *self = M7CLMidi::YesMidiNoConnection(midi, ports, None)
                }
                Err(init_error) => *self = M7CLMidi::NoMidi(init_error),
            }
        } else {
            // TODO: implement logging and log that someone did an oopsie
        }
    }
    /// For a connection with MIDI initialized but not connected, return the list of available
    /// ports
    pub fn ports(&self) -> Result<&midir::MidiOutputPorts, ()> {
        if let M7CLMidi::YesMidiNoConnection(_, ports, _) = self {
            Ok(&ports)
        } else {
            // TODO: logging and someone did an oopsie
            Err(())
        }
    }
    /// For a connection with MIDI initialized but not connected, update the list of MIDI output ports
    pub fn update_ports_list(&mut self) {
        if let M7CLMidi::YesMidiNoConnection(midi, ports, _) = self {
            *ports = midi.ports();
        } else {
            // TODO: logging and say someone did an oopsie
        }
    }
    /// For a connection with MIDI initialized but not connected, try to connect to the given port
    pub fn try_connect(&mut self, port: &MidiOutputPort) {
        // Because `connect` needs ownership of the midi object, we must temporarily take `self`.
        // Note that we must then return `self` from the closure.
        take_mut::take(self, |self_| {
            if let M7CLMidi::YesMidiNoConnection(midi, _, _) = self_ {
                let connection = midi.connect(port, MIDI_CONNECTION_NAME);
                match connection {
                    Ok(connection) => M7CLMidi::Connected(connection),
                    Err(connection_error) => {
                        let error = connection_error.kind();
                        let midi = connection_error.into_inner();
                        let ports = midi.ports();
                        M7CLMidi::YesMidiNoConnection(midi, ports, Some(error))
                    }
                }
            } else {
                //TODO: logging and oopsie
                self_
            }
        });
    }
}
