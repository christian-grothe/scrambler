pub use triple_buffer::{triple_buffer, Input, Output};

pub const SEQUENCES: i32 = 3;
pub const STEP_NUM: u8 = 8;
pub const INIT_BPM: f32 = 60.0;
pub const AUDIO_BUFFER_SIZE_SEC: f32 = 5.0;
pub const VOICE_NUM: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Subdivision {
    Half,             // Half note (1/2)
    Quarter,          // Quarter note (1/4)
    Eighth,           // Eighth note (1/8)
    Sixteenth,        // Sixteenth note (1/16)
    ThirtySecond,     // Thirty-second note (1/32)
    TripletQuarter,   // Quarter-note triplet
    TripletEighth,    // Eighth-note triplet
    TripletSixteenth, // Sixteenth-note triplet
    DottedQuarter,    // Quarter note * 1.5
    DottedEighth,     // Eighth note * 1.5
    DottedSixteenth,  // Sixteenth note * 1.5
}

impl Subdivision {
    fn factor(self) -> f32 {
        match self {
            Subdivision::Quarter => 1.0,
            Subdivision::Half => 2.0,
            Subdivision::Eighth => 0.5,
            Subdivision::Sixteenth => 0.25,
            Subdivision::ThirtySecond => 0.125,
            Subdivision::TripletQuarter => 2.0 / 3.0,
            Subdivision::TripletEighth => 1.0 / 3.0,
            Subdivision::TripletSixteenth => 1.0 / 6.0,
            Subdivision::DottedQuarter => 1.5,
            Subdivision::DottedEighth => 0.75,
            Subdivision::DottedSixteenth => 0.375,
        }
    }

    fn to_hz(self, bpm: f32) -> f32 {
        (bpm / 60.0) / self.factor()
    }
}

#[derive(Clone)]
pub struct DrawData {
    pub positions: Vec<u8>,
    pub bpm: f32,
    pub transporter: (u8, u8, u8),
}

impl DrawData {
    fn new() -> Self {
        DrawData {
            positions: vec![0; SEQUENCES as usize],
            bpm: INIT_BPM,
            transporter: (0, 0, 0),
        }
    }
}

pub struct Sequencer {
    bpm: f32,
    sequences: Vec<Sequence>,
    draw_data: Input<DrawData>,
    steps: Vec<Step>,
    transporter: Transporter,
}

impl Sequencer {
    pub fn new(sample_rate: f32) -> (Self, Output<DrawData>) {
        let (buf_input, buf_output) = triple_buffer(&DrawData::new());
        let bpm = INIT_BPM;
        (
            Sequencer {
                bpm,
                draw_data: buf_input,
                sequences: {
                    let mut sequences: Vec<Sequence> = Vec::with_capacity(SEQUENCES as usize);
                    sequences.push(Sequence::new(sample_rate, bpm, Subdivision::Quarter));
                    sequences.push(Sequence::new(sample_rate, bpm, Subdivision::TripletEighth));
                    sequences.push(Sequence::new(sample_rate, bpm, Subdivision::TripletQuarter));
                    // sequences.push(Sequence::new(
                    //     sample_rate,
                    //     bpm,
                    //     Subdivision::DottedSixteenth,
                    // ));
                    sequences
                },
                steps: {
                    let mut steps: Vec<Step> = Vec::with_capacity(STEP_NUM as usize);
                    for _ in 0..STEP_NUM {
                        steps.push(Step::new(sample_rate))
                    }
                    steps
                },
                transporter: Transporter::new(sample_rate),
            },
            buf_output,
        )
    }

    pub fn render(&mut self, sample: &mut f32) {
        self.transporter.update();

        let draw_data = self.draw_data.input_buffer();
        let positions = &mut draw_data.positions;
        let bpm = &mut draw_data.bpm;
        let transporter = &mut draw_data.transporter;

        for step in self.steps.iter_mut() {
            if step.is_recording {
                step.record(*sample);
            }
        }

        for (i, sequence) in self.sequences.iter_mut().enumerate() {
            if let Some(step) = sequence.update() {
                self.steps[step as usize].play();
            }
            positions[i] = sequence.current_step;
        }
        *transporter = (
            self.transporter.quater,
            self.transporter.eights,
            self.transporter.sixteenth,
        );
        *bpm = self.bpm;
        self.draw_data.publish();

        let mut output = 0.0;
        for step in self.steps.iter_mut() {
            output += step.render();
        }

        *sample = output;
    }

    pub fn record(&mut self, step_idx: usize) {
        if let Some(step) = self.steps.get_mut(step_idx) {
            step.is_recording = true;
            step.record_head = 0;
        }
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm;
        for sequence in self.sequences.iter_mut() {
            sequence.set_bpm(bpm);
        }
    }

    pub fn set_subdivision(&mut self, index: usize, subdivision: Subdivision) {
        if let Some(instance) = self.sequences.get_mut(index) {
            instance.set_subdivision(self.bpm, subdivision);
        }
    }
}

struct Transporter {
    quater: u8,
    eights: u8,
    sixteenth: u8,
    counter: Counter,
}

impl Transporter {
    fn new(sample_rate: f32) -> Self {
        let freq = Subdivision::Sixteenth.to_hz(INIT_BPM);

        Transporter {
            quater: 0,
            eights: 0,
            sixteenth: 0,
            counter: Counter::new(sample_rate, freq / sample_rate),
        }
    }

    fn update(&mut self) {
        if self.counter.update() {
            self.sixteenth += 1;
            if self.sixteenth > 16 {
                self.sixteenth = 0;
                self.eights += 1;
            }
            if self.eights > 8 {
                self.eights = 0;
                self.quater += 1;
            }
            if self.quater > 4 {
                self.quater = 0;
            }
        }
    }
}

#[derive(PartialEq)]
enum EnvState {
    Attack,
    Release,
}

struct Voice {
    play_head: usize,
    is_playing: bool,
    gain: f32,
    gain_inc_attack: f32,
    gain_inc_release: f32,
    env_state: EnvState,
    buffer_size: usize,
    pitch: f32,
}

impl Voice {
    fn new(sample_rate: f32, buffer_size: usize) -> Self {
        Voice {
            play_head: 0,
            is_playing: false,
            gain: 0.0,
            gain_inc_attack: 50.0 / sample_rate,
            gain_inc_release: 0.5 / sample_rate,
            env_state: EnvState::Attack,
            buffer_size,
            pitch: 2.0,
        }
    }

    fn render(&mut self) -> (usize, f32) {
        match self.env_state {
            EnvState::Attack => self.gain += self.gain_inc_attack,
            EnvState::Release => self.gain -= self.gain_inc_release,
        };

        if self.gain >= 1.0 {
            self.env_state = EnvState::Release;
        }

        if self.gain <= 0.0 && self.env_state == EnvState::Release {
            self.gain = 0.0;
        }

        self.play_head += 1;
        if self.play_head >= self.buffer_size {
            self.is_playing = false;
            self.play_head = 0;
            self.env_state = EnvState::Attack;
        }

        (self.play_head, self.gain)
    }
}

struct Step {
    buffer: Vec<f32>,
    record_head: usize,
    _sample_rate: f32,
    is_recording: bool,
    voices: Vec<Voice>,
}

impl Step {
    fn new(sample_rate: f32) -> Self {
        let buffer_size = sample_rate * AUDIO_BUFFER_SIZE_SEC;
        Step {
            buffer: vec![0.0; buffer_size as usize],
            record_head: 0,
            _sample_rate: sample_rate,
            is_recording: false,
            voices: {
                let mut voices: Vec<Voice> = Vec::with_capacity(VOICE_NUM as usize);
                for _ in 0..VOICE_NUM {
                    voices.push(Voice::new(sample_rate, buffer_size as usize));
                }
                voices
            },
        }
    }

    fn record(&mut self, sample: f32) {
        self.buffer[self.record_head] = sample;
        self.record_head += 1;
        if self.record_head >= self.buffer.len() {
            self.is_recording = false;
            self.record_head = 0;
        }
    }

    fn play(&mut self) {
        for voice in self.voices.iter_mut() {
            if !voice.is_playing {
                voice.is_playing = true;
                break;
            }
        }
    }

    fn render(&mut self) -> f32 {
        let mut sample = 0.0;
        for voice in self.voices.iter_mut() {
            if voice.is_playing {
                let (pos, gain) = voice.render();
                sample += self.buffer[pos] * gain;
            }
        }
        sample
    }
}
struct Sequence {
    subdivision: Subdivision,
    counter: Counter,
    current_step: u8,
}

impl Sequence {
    fn new(sample_rate: f32, bpm: f32, subdivision: Subdivision) -> Self {
        Sequence {
            subdivision,
            counter: Counter::new(sample_rate, subdivision.to_hz(bpm)),
            current_step: 0,
        }
    }

    fn update(&mut self) -> Option<u8> {
        if self.counter.update() {
            self.current_step = self.current_step + 1;
            if self.current_step >= STEP_NUM {
                self.current_step = 0;
            }
            Some(self.current_step)
        } else {
            None
        }
    }

    fn set_bpm(&mut self, bpm: f32) {
        self.counter.set_frequency(self.subdivision.to_hz(bpm));
    }

    fn set_subdivision(&mut self, current_bpm: f32, subdivision: Subdivision) {
        self.subdivision = subdivision;
        self.set_bpm(current_bpm);
    }
}

struct Counter {
    phase: f32,
    increment: f32,
    sample_rate: f32,
}

impl Counter {
    fn new(sample_rate: f32, frequency: f32) -> Self {
        Counter {
            phase: 0.0,
            increment: frequency / sample_rate,
            sample_rate,
        }
    }

    fn update(&mut self) -> bool {
        self.phase = self.phase + self.increment;
        if self.phase >= 1.0 {
            self.phase = 0.0;
            return true;
        }
        return false;
    }

    fn set_frequency(&mut self, freq: f32) {
        self.increment = freq / self.sample_rate;
    }
}
