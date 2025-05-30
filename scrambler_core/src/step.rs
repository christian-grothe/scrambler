use crate::{voice::Voice, AUDIO_BUFFER_SIZE_SEC, VOICE_NUM};

#[derive(PartialEq, Clone)]
pub enum StepState {
    Recording,
    Recorded,
    Empty,
}

impl StepState {
    pub fn get_symbol(&self) -> &str {
        match self {
            StepState::Recording => "  ●  ",
            StepState::Recorded => "  ○  ",
            StepState::Empty => "  -  ",
        }
    }
}

pub struct Step {
    buffer: Vec<f32>,
    pub record_head: usize,
    voices: Vec<Voice>,
    pub state: StepState,
}

impl Step {
    pub fn new(sample_rate: f32) -> Self {
        let buffer_size = sample_rate * AUDIO_BUFFER_SIZE_SEC;
        Step {
            buffer: vec![0.0; buffer_size as usize],
            record_head: 0,
            voices: {
                let mut voices: Vec<Voice> = Vec::with_capacity(VOICE_NUM as usize);
                for _ in 0..VOICE_NUM {
                    voices.push(Voice::new(sample_rate, buffer_size as usize));
                }
                voices
            },
            state: StepState::Empty,
        }
    }

    pub fn set_attack(&mut self, val: f32) {
        for voice in self.voices.iter_mut() {
            voice.set_attack(val);
        }
    }

    pub fn set_release(&mut self, val: f32) {
        for voice in self.voices.iter_mut() {
            voice.set_release(val);
        }
    }

    pub fn record(&mut self, sample: f32) {
        self.buffer[self.record_head] = sample;
        self.record_head += 1;
        if self.record_head >= self.buffer.len() {
            self.state = StepState::Recorded;
            self.record_head = 0;
        }
    }

    pub fn erase(&mut self) {
        self.buffer.fill_with(Default::default);
        self.state = StepState::Empty;
        self.record_head = 0;
    }

    pub fn play(&mut self, pitch: f32, gain: f32, attack: f32, release: f32) {
        self.set_attack(attack);
        self.set_release(release);

        for voice in self.voices.iter_mut() {
            if !voice.is_playing {
                voice.pitch = pitch;
                voice.gain = gain;
                voice.is_playing = true;
                break;
            }
        }
    }

    pub fn render(&mut self) -> f32 {
        let mut sample = 0.0;
        for voice in self.voices.iter_mut() {
            if voice.is_playing {
                let (pos, env, gain) = voice.render();
                let pos_int = pos as usize;
                let next_pos = (pos_int + 1) % self.buffer.len();
                let frac = pos - pos_int as f32;

                let next_sample =
                    self.buffer[pos_int] * (1.0 - frac) + self.buffer[next_pos] * frac;
                sample += next_sample * env * gain;
            }
        }
        sample
    }
}
