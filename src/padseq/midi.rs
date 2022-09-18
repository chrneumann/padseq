extern crate midir;

use std::collections::VecDeque;
use std::sync::mpsc;

use std::thread::sleep;
use std::time::{Duration, Instant};

use super::session::{Note, Velocity};
use midir::{Ignore, MidiIO, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

pub struct MidiEvent {
    pub message: MidiMessage,
    pub instant: Option<Instant>,
}

pub enum MidiMessageType {
    NoteOff,
    NoteOn,
}

pub struct MidiMessage {
    pub r#type: MidiMessageType,
    pub note: u8,
    pub velocity: u8,
}

impl MidiMessage {
    pub fn to_array(&self) -> [u8; 3] {
        let the_type = match self.r#type {
            MidiMessageType::NoteOff => 0x80,
            MidiMessageType::NoteOn => 0x90,
        };
        return [the_type, self.note, self.velocity];
    }

    pub fn from_array(message: &[u8]) -> MidiMessage {
        return MidiMessage {
            r#type: match message[2] {
                0 => MidiMessageType::NoteOff,
                _ => MidiMessageType::NoteOn,
            },
            note: message[1],
            velocity: message[2],
        };
    }
}

type MidiEventQueue = VecDeque<MidiEvent>;

pub struct Instrument {
    name: &'static str,
    midi_out: Option<MidiOutputConnection>,
    midi_in: Option<MidiInputConnection<mpsc::Sender<MidiEvent>>>,
    events_in: MidiEventQueue,
    events_out: MidiEventQueue,
    chan_out: mpsc::Sender<MidiEvent>,
    chan_in: mpsc::Receiver<MidiEvent>,
}

impl Instrument {
    pub fn new(name: &'static str) -> Instrument {
        let (tx, rx) = mpsc::channel();
        Instrument {
            midi_out: None,
            midi_in: None,
            events_in: VecDeque::new(),
            events_out: VecDeque::new(),
            chan_in: rx,
            chan_out: tx,
            name: name,
        }
    }

    pub fn send_events(&mut self) {
        for _ in 0..self.events_out.len() {
            let message = self.events_out.pop_front();
            match message {
                Some(x) => {
                    if x.instant == None || Instant::now() > x.instant.unwrap() {
                        match &mut self.midi_out {
                            Some(out) => {
                                println!("send {:?}", &x.message.to_array());
                                let _ = out.send(&x.message.to_array());
                            }
                            None => {}
                        }
                    } else {
                        self.events_out.insert(0, x);
                    }
                }
                None => {}
            }
        }
    }

    pub fn play_note(&mut self, note: Note, velocity: Velocity, duration: u64) {
        {
            println!("play on note {} {} {}", note, velocity, duration);
            let message = MidiMessage {
                r#type: MidiMessageType::NoteOn,
                note: note,
                velocity: velocity,
            };
            self.push_event(MidiEvent {
                message: message,
                instant: None,
            });
        }
        if duration > 0 {
            println!("also play stop note>");
            let message = MidiMessage {
                r#type: MidiMessageType::NoteOff,
                note: note,
                velocity: 0,
            };
            self.push_event(MidiEvent {
                message: message,
                instant: Instant::now().checked_add(Duration::from_millis(duration)),
            });
        }
    }

    pub fn stop_note(&mut self, note: Note) {
        println!("stop note {}", note);
        let message = MidiMessage {
            r#type: MidiMessageType::NoteOff,
            note: note,
            velocity: 0,
        };
        self.push_event(MidiEvent {
            message: message,
            instant: None,
        });
    }

    fn receive_events(&mut self) {
        loop {
            match self.chan_in.try_recv() {
                Ok(t) => self.events_in.push_back(t),
                Err(e) => match e {
                    mpsc::TryRecvError::Empty => break,
                    mpsc::TryRecvError::Disconnected => panic!("Channel died"),
                },
            }
        }
    }

    pub fn has_events(&mut self) -> bool {
        self.receive_events();
        return self.events_in.len() > 0;
    }

    pub fn pop_event(&mut self) -> Option<MidiEvent> {
        self.receive_events();
        let element = self.events_in.pop_front();
        return element;
    }

    fn push_event(&mut self, event: MidiEvent) {
        return self.events_out.push_back(event);
    }

    pub fn connect_out(&mut self, port: u8) {
        let midi_out = MidiOutput::new(self.name).unwrap();
        let out_port = self.select_port(port, &midi_out).unwrap();
        let port_name = midi_out.port_name(&out_port).unwrap();
        println!("Connection open, outgoing to '{}' ...", port_name);
        let conn_out = midi_out.connect(&out_port, self.name).unwrap();
        self.midi_out = Some(conn_out);
    }

    pub fn connect_in(&mut self, port: u8) {
        let mut midi_in = MidiInput::new("instrument").unwrap();
        midi_in.ignore(Ignore::None);
        let in_port = self.select_port(port, &midi_in).unwrap();
        let port_name = midi_in.port_name(&in_port).unwrap();
        println!("Connection open, incoming from '{}' ...", port_name);

        // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
        self.midi_in = Some(
            midi_in
                .connect(
                    &in_port,
                    "midir-forward",
                    |stamp, message, chan_out| {
                        // conn_out.send(message).unwrap_or_else(|_| println!("Error when forwarding message ..."));
                        println!("{}: {:?} (len = {})", stamp, message, message.len());
                        // let value : usize = message[1] as usize;
                        chan_out.send(MidiEvent {
                            message: MidiMessage::from_array(message),
                            instant: None,
                        });
                    },
                    self.chan_out.clone(),
                )
                .unwrap(),
        );
    }

    fn select_port<T: MidiIO>(&self, force: u8, midi_io: &T) -> Result<T::Port, ()> {
        // println!("Available {} ports:", descr);
        let midi_ports = midi_io.ports();
        // for (i, p) in midi_ports.iter().enumerate() {
        //     println!("{}: {}", i, midi_io.port_name(p)?);
        // }
        // print!("Please select {} port: ", descr);
        // stdout().flush()?;
        // let mut input = String::new();
        // stdin().read_line(&mut input)?;
        // let port = midi_ports.get(input.trim().parse::<usize>()?)
        //                      .ok_or("Invalid port number")?;
        let port = midi_ports
            .get(force as usize)
            .ok_or("Invalid port number")
            .unwrap();
        Ok(port.clone())
    }
}