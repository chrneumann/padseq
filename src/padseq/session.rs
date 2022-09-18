pub type Step = u8;
pub type Note = u8;
pub type Velocity = u8;
pub type Bar = [Velocity; 8];

pub struct Session {
    bar: Bar,
}

impl Session {
    pub fn new() -> Session {
        Session {
            bar: [0, 0, 0, 0, 0, 0, 0, 0],
        }
    }

    pub fn set_step(&mut self, step: Step, note: Note) {
        self.bar[step as usize] = note;
    }

    pub fn get_step(&self, step: Step) -> Note {
        println!("{:?} {} -> {}", self.bar, step, self.bar[step as usize]);
        return self.bar[step as usize];
    }
}