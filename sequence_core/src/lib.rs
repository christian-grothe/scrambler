pub use triple_buffer::{triple_buffer, Input, Output};

pub const SEQUENCES: i32 = 3;
pub const STEP_NUM: u8 = 8;
pub const INIT_BPM: f32 = 100.0;
pub const AUDIO_BUFFER_SIZE_SEC: f32 = 5.0;
pub const VOICE_NUM: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn get_symbol(&self) -> &str {
        match self {
            Subdivision::Quarter => "1/4",
            Subdivision::Half => "2/4",
            Subdivision::Eighth => "1/8",
            Subdivision::Sixteenth => "1/16",
            Subdivision::ThirtySecond => "1/32",
            Subdivision::TripletQuarter => "1/4t",
            Subdivision::TripletEighth => "1/8t",
            Subdivision::TripletSixteenth => "1/16t",
            Subdivision::DottedQuarter => "1/4.",
            Subdivision::DottedEighth => "1/8.",
            Subdivision::DottedSixteenth => "1/16.",
        }
    }
}

#[derive(Clone)]
pub struct DrawData {
    pub positions: Vec<u8>,
    pub subdivisions: Vec<Subdivision>,
    pub pitches: Vec<f32>,
    pub ranges: Vec<(u8, u8)>,
    pub bpm: f32,
    pub transporter: (u8, u8, u8),
}

impl DrawData {
    fn new() -> Self {
        DrawData {
            positions: vec![0; SEQUENCES as usize],
            bpm: INIT_BPM,
            transporter: (0, 0, 0),
            subdivisions: vec![Subdivision::Quarter; SEQUENCES as usize],
            pitches: vec![1.0; SEQUENCES as usize],
            ranges: vec![(0, 0); SEQUENCES as usize],
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
                    sequences.push(Sequence::new(
                        sample_rate,
                        bpm,
                        Subdivision::Quarter,
                        1.0,
                        (0, 4),
                        PlayMode::Forwards,
                    ));
                    sequences.push(Sequence::new(
                        sample_rate,
                        bpm,
                        Subdivision::Eighth,
                        1.5,
                        (3, 6),
                        PlayMode::Backwards,
                    ));
                    sequences.push(Sequence::new(
                        sample_rate,
                        bpm,
                        Subdivision::Sixteenth,
                        2.0,
                        (4, 7),
                        PlayMode::BackAndForth(0),
                    ));
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
        let draw_data = self.draw_data.input_buffer();
        let positions = &mut draw_data.positions;
        let subdivisions = &mut draw_data.subdivisions;
        let bpm = &mut draw_data.bpm;
        let transporter = &mut draw_data.transporter;
        let pitches = &mut draw_data.pitches;
        let ranges = &mut draw_data.ranges;

        for step in self.steps.iter_mut() {
            if step.is_recording {
                step.record(*sample);
            }
        }

        let apply = self.transporter.update();
        for (i, sequence) in self.sequences.iter_mut().enumerate() {
            if let Some((step, pitch)) = sequence.update(apply, self.bpm) {
                self.steps[step as usize].play(pitch);
            }
            positions[i] = sequence.current_step;
            pitches[i] = sequence.pitch;
            ranges[i] = sequence.play_range;
            if let Some(subdivision) = sequence.next_subdivision {
                subdivisions[i] = subdivision;
            } else {
                subdivisions[i] = sequence.subdivision;
            }
        }

        *transporter = (
            self.transporter.bar,
            self.transporter.quater,
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

    pub fn set_pitch(&mut self, idx: usize, pitch: f32) {
        if let Some(sequence) = self.sequences.get_mut(idx) {
            sequence.pitch = pitch;
        }
    }

    pub fn toggle(&mut self, idx: usize) {
        if let Some(sequence) = self.sequences.get_mut(idx) {
            sequence.toggle();
        }
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm;
        self.transporter.set_bpm(bpm);
        for sequence in self.sequences.iter_mut() {
            sequence.set_bpm(bpm);
        }
    }

    pub fn set_subdivision(&mut self, index: usize, subdivision: Subdivision) {
        if let Some(sequence) = self.sequences.get_mut(index) {
            sequence.set_subdivision(subdivision);
        }
    }
}

struct Transporter {
    bar: u8,
    quater: u8,
    sixteenth: u8,
    counter: Counter,
    quantisation: Subdivision,
}

impl Transporter {
    fn new(sample_rate: f32) -> Self {
        let freq = Subdivision::Sixteenth.to_hz(INIT_BPM);

        Transporter {
            quater: 0,
            bar: 0,
            sixteenth: 0,
            counter: Counter::new(sample_rate, freq),
            quantisation: Subdivision::Quarter,
        }
    }

    fn update(&mut self) -> bool {
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

    fn set_bpm(&mut self, bpm: f32) {
        self.counter
            .set_frequency(Subdivision::Sixteenth.to_hz(bpm))
    }
}

#[derive(PartialEq)]
enum EnvState {
    Attack,
    Release,
}

struct Voice {
    play_head: f32,
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
            play_head: 0.0,
            is_playing: false,
            gain: 0.0,
            gain_inc_attack: 50.0 / sample_rate,
            gain_inc_release: 5.0 / sample_rate,
            env_state: EnvState::Attack,
            buffer_size,
            pitch: 4.0,
        }
    }

    fn render(&mut self) -> (f32, f32) {
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

        self.play_head += self.pitch;
        if self.play_head >= self.buffer_size as f32 {
            self.is_playing = false;
            self.play_head = 0.0;
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

    fn play(&mut self, pitch: f32) {
        for voice in self.voices.iter_mut() {
            if !voice.is_playing {
                voice.pitch = pitch;
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
                let pos_int = pos as usize;
                let next_pos = (pos_int + 1) % self.buffer.len();
                let frac = pos - pos_int as f32;

                let next_sample =
                    self.buffer[pos_int] * (1.0 - frac) + self.buffer[next_pos] * frac;
                sample += next_sample * gain;
            }
        }
        sample
    }
}

#[derive(PartialEq)]
enum PlayState {
    Playing,
    Stopped,
    Resume,
}

enum PlayMode {
    Forwards,
    Backwards,
    BackAndForth(u8),
}

struct Sequence {
    subdivision: Subdivision,
    next_subdivision: Option<Subdivision>,
    counter: Counter,
    current_step: u8,
    pitch: f32,
    play_state: PlayState,
    play_mode: PlayMode,
    play_range: (u8, u8),
}

impl Sequence {
    fn new(
        sample_rate: f32,
        bpm: f32,
        subdivision: Subdivision,
        pitch: f32,
        play_range: (u8, u8),
        play_mode: PlayMode,
    ) -> Self {
        Sequence {
            subdivision,
            next_subdivision: None,
            counter: Counter::new(sample_rate, subdivision.to_hz(bpm)),
            current_step: play_range.0,
            pitch,
            play_state: PlayState::Stopped,
            play_mode,
            play_range,
        }
    }

    fn toggle(&mut self) {
        match self.play_state {
            PlayState::Playing => {
                self.play_state = PlayState::Stopped;
                self.counter.reset();
            }
            PlayState::Stopped => self.play_state = PlayState::Resume,
            PlayState::Resume => self.play_state = PlayState::Stopped,
        }
    }

    fn update(&mut self, apply_change: bool, current_bpm: f32) -> Option<(u8, f32)> {
        if apply_change {
            if self.play_state == PlayState::Resume {
                self.play_state = PlayState::Playing;
            }

            self.apply_subdivision(current_bpm);
        }

        if self.play_state != PlayState::Playing || !self.counter.update() {
            None
        } else {
            match self.play_mode {
                PlayMode::Forwards => {
                    self.current_step += 1;
                    if self.current_step > self.play_range.1 {
                        self.current_step = self.play_range.0;
                    }
                }
                PlayMode::Backwards => {
                    self.current_step -= 1;
                    if self.current_step < self.play_range.0 {
                        self.current_step = self.play_range.1;
                    }
                }
                PlayMode::BackAndForth(id) => {
                    if id == 0 {
                        self.current_step += 1;
                        if self.current_step >= self.play_range.1 {
                            self.play_mode = PlayMode::BackAndForth(1);
                        }
                    } else {
                        self.current_step -= 1;
                        if self.current_step <= self.play_range.0 {
                            self.play_mode = PlayMode::BackAndForth(0);
                        }
                    }
                }
            }
            Some((self.current_step, self.pitch))
        }
    }

    fn set_bpm(&mut self, bpm: f32) {
        self.counter.set_frequency(self.subdivision.to_hz(bpm));
    }

    fn apply_subdivision(&mut self, current_bpm: f32) {
        if let Some(subdivision) = self.next_subdivision {
            self.subdivision = subdivision;
            self.set_bpm(current_bpm);
            self.counter.reset();
            self.next_subdivision = None;
        }
    }

    fn set_subdivision(&mut self, subdivision: Subdivision) {
        self.next_subdivision = Some(subdivision);
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

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn update(&mut self) -> bool {
        self.phase += self.increment;
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
