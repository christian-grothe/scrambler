pub struct Counter {
    phase: f32,
    increment: f32,
    sample_rate: f32,
}

impl Counter {
    pub fn new(sample_rate: f32, frequency: f32) -> Self {
        Counter {
            phase: 0.0,
            increment: frequency / sample_rate,
            sample_rate,
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn update(&mut self) -> bool {
        self.phase += self.increment;
        if self.phase >= 1.0 {
            self.phase = 0.0;
            return true;
        }
        return false;
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.increment = freq / self.sample_rate;
    }
}
