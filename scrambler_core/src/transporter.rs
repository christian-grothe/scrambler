use crate::{Counter, Subdivision, INIT_BPM};

pub struct Transporter {
    pub bar: u8,
    pub quater: u8,
    pub sixteenth: u8,
    counter: Counter,
    quantisation: Subdivision,
}

impl Transporter {
    pub fn new(sample_rate: f32) -> Self {
        let freq = Subdivision::Sixteenth.to_hz(INIT_BPM);

        Transporter {
            quater: 0,
            bar: 0,
            sixteenth: 0,
            counter: Counter::new(sample_rate, freq),
            quantisation: Subdivision::Quarter,
        }
    }

    pub fn update(&mut self) -> bool {
        let mut ret: bool = false;
        if self.counter.update() {
            self.sixteenth += 1;
            if self.quantisation == Subdivision::Sixteenth {
                ret = true;
            }
            if self.sixteenth >= 4 {
                self.sixteenth = 0;
                self.quater += 1;
                if self.quantisation == Subdivision::Quarter {
                    ret = true;
                }
            }
            if self.quater >= 4 {
                self.quater = 0;
                self.bar += 1;
            }
            if self.bar >= 4 {
                self.bar = 0;
            }
        }
        ret
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.counter
            .set_frequency(Subdivision::Sixteenth.to_hz(bpm))
    }
}
