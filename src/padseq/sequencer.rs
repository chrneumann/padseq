use super::midi::{Instrument, MidiMessageType};
use super::session::{Note, Session, Step, StepNotes, BAR_SIZE};
use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

const PAD_BAR_NOTES: [Note; BAR_SIZE as usize] = [
    81, 82, 83, 84, 85, 86, 87, 88, 71, 72, 73, 74, 75, 76, 77, 78, 61, 62, 63, 64, 65, 66, 67, 68,
    51, 52, 53, 54, 55, 56, 57, 58,
];
const PAD_KEY_NOTES: [Note; 12] = [22, 32, 23, 33, 24, 25, 35, 26, 36, 27, 37, 28];
const PAD_COLOR_STEP_OFF: u8 = 112;
const PAD_COLOR_STEP_SET: u8 = 45;
const PAD_COLOR_STEP_SET_AND_ACTIVE: u8 = 78;
const PAD_COLOR_STEP_ACTIVE: u8 = 3;
const PAD_COLOR_KEY: u8 = 12;
const PAD_COLOR_KEY_ACTIVE: u8 = 9;
const BASE_C_NOTE: Note = 60;

type StepSize = f64;
const BASE_BPM: StepSize = 126.0;

pub struct Sequencer {
    session: Session,
    pad: Instrument,
    instrument: Instrument,
    selected_notes: HashSet<Note>,
    active: Step,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        Sequencer {
            session: Session::new(),
            pad: Instrument::new("Pad"),
            instrument: Instrument::new("Instrument"),
            active: 0,
            selected_notes: HashSet::new(),
            // instruments: Vec<Instrument>::new(),
        }
    }

    pub fn connect(&mut self) {
        self.pad.connect_out(2);
        self.pad.connect_in(2);
        self.instrument.connect_out(0);
        print!("Connect done");
    }

    pub fn handle_pad_events(&mut self) {
        while self.pad.has_events() {
            let event = self.pad.pop_event().unwrap();
            let message = event.message;
            let note = message.note;
            if PAD_KEY_NOTES.contains(&note) {
                let keyNote =
                    BASE_C_NOTE + PAD_KEY_NOTES.iter().position(|&x| x == note).unwrap() as Note;
                match message.r#type {
                    MidiMessageType::NoteOn => {
                        self.instrument.play_note(1, keyNote, 127, 0);
                        self.pad.play_note(1, note, PAD_COLOR_KEY_ACTIVE, 0);
                        self.selected_notes.insert(keyNote);
                    }
                    MidiMessageType::NoteOff => {
                        self.instrument.stop_note(1, keyNote);
                        self.pad.play_note(1, note, PAD_COLOR_KEY, 0);
                        self.selected_notes.remove(&keyNote);
                    }
                }
            } else if PAD_BAR_NOTES.contains(&note) {
                if message.velocity > 0 {
                    let step = PAD_BAR_NOTES.iter().position(|&x| x == note).unwrap() as Step;
                    if self.selected_notes.len() == 0 {
                        self.session.clear_step(step);
                    } else {
                        let mut step_notes = if self.session.has_step_set(step) {
                            self.session.get_step(step).clone()
                        } else {
                            StepNotes::new()
                        };
                        for note in &self.selected_notes {
                            if step_notes.contains_key(note) {
                                step_notes.remove(note);
                            } else {
                                step_notes.insert(*note, 127);
                            }
                        }
                        self.session.set_step(step, &step_notes);
                    }
                    // println!("set note {} {}", note, _note);
                    // self.session.set_step(step, _note);
                }
            }
        }
    }

    fn play_notes(&mut self) {
        if self.session.has_step_set(self.active) {
            let notes = self.session.get_step(self.active);
            for (note, velocity) in notes {
                println!("play {}", note);
                self.instrument.play_note(1, *note, *velocity, 0);
            }
        }
    }

    fn refresh_step(&mut self, step: Step) {
        let note = PAD_BAR_NOTES[step as usize];
        let channel = if step == 0 || step == BAR_SIZE - 1 {
            3
        } else {
            1
        };
        if step == self.active {
            if self.session.has_step_set(step) {
                self.pad
                    .play_note(channel, note, PAD_COLOR_STEP_SET_AND_ACTIVE, 0);
            } else {
                self.pad.play_note(channel, note, PAD_COLOR_STEP_ACTIVE, 0);
            }
        } else {
            if self.session.has_step_set(step) {
                self.pad.play_note(channel, note, PAD_COLOR_STEP_SET, 0);
            } else {
                self.pad.play_note(channel, note, PAD_COLOR_STEP_OFF, 0);
            }
        }
    }

    fn refresh(&mut self) {
        for n in 0..BAR_SIZE {
            self.refresh_step(n);
        }
    }

    pub fn run(&mut self) {
        self.instrument.set_debug(true);
        let mut step = 0.0;
        for n in 0..12 {
            let note = PAD_KEY_NOTES[n];
            self.pad.play_note(1, note, PAD_COLOR_KEY, 0);
        }
        loop {
            if step >= 1000.0 * 60.0 / (4.0 * BASE_BPM) {
                self.active = (self.active + 1) % BAR_SIZE;
                step = 0.0;
                self.play_notes();
                self.refresh();
            }
            self.instrument.send_events();
            self.handle_pad_events();
            self.pad.send_events();
            sleep(Duration::from_millis(10));
            step += 10.0;
        }
    }
}
