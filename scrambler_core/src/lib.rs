mod constants;
mod counter;
mod sequence;
mod step;
mod subdivision;
mod transporter;
mod voice;

pub use constants::*;
use counter::*;
pub use sequence::*;
use step::*;
pub use subdivision::*;
use transporter::*;
pub use triple_buffer::{triple_buffer, Input, Output};

#[derive(Clone)]
pub struct DrawData {
    pub positions: Vec<u8>,
    pub subdivisions: Vec<Subdivision>,
    pub pitches: Vec<f32>,
    pub ranges: Vec<(u8, u8)>,
    pub dirs: Vec<PlayMode>,
    pub step_states: Vec<StepState>,
    pub bpm: f32,
    pub transporter: (u8, u8, u8),
    pub gains: Vec<f32>,
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
            dirs: vec![PlayMode::Forwards; SEQUENCES as usize],
            step_states: vec![StepState::Empty; STEP_NUM as usize],
            gains: vec![0.8; SEQUENCES as usize],
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
                        1.0,
                        (3, 6),
                        PlayMode::Backwards,
                    ));
                    sequences.push(Sequence::new(
                        sample_rate,
                        bpm,
                        Subdivision::Sixteenth,
                        1.0,
                        (4, 7),
                        PlayMode::BackAndForth(0),
                    ));
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
        let dirs = &mut draw_data.dirs;
        let step_states = &mut draw_data.step_states;
        let gains = &mut draw_data.gains;

        for step in self.steps.iter_mut() {
            if step.state == StepState::Recording {
                step.record(*sample);
            }
        }

        let mut output = 0.0;

        let apply = self.transporter.update();
        for (i, sequence) in self.sequences.iter_mut().enumerate() {
            if let Some((step, pitch, gain)) = sequence.update(apply, self.bpm) {
                self.steps[step as usize].play(pitch, gain);
            }
            positions[i] = sequence.current_step;
            pitches[i] = sequence.pitch;
            ranges[i] = sequence.play_range;
            dirs[i] = sequence.play_mode.clone();
            gains[i] = sequence.gain;

            if let Some(subdivision) = sequence.next_subdivision {
                subdivisions[i] = subdivision;
            } else {
                subdivisions[i] = sequence.subdivision;
            }
        }

        for (i, step) in self.steps.iter().enumerate() {
            step_states[i] = step.state.clone();
        }

        *transporter = (
            self.transporter.bar,
            self.transporter.quater,
            self.transporter.sixteenth,
        );
        *bpm = self.bpm;
        self.draw_data.publish();

        for step in self.steps.iter_mut() {
            output += step.render();
        }

        *sample = output;
    }

    pub fn record(&mut self, step_idx: usize) {
        if let Some(step) = self.steps.get_mut(step_idx) {
            step.state = StepState::Recording;
            step.record_head = 0;
        }
    }

    pub fn erase(&mut self, step_idx: usize) {
        if let Some(step) = self.steps.get_mut(step_idx) {
            step.erase();
        }
    }

    pub fn set_pitch(&mut self, idx: usize, semitone: i8) {
        let pitch = 2.0f32.powf(semitone as f32 / 12.0);
        if let Some(sequence) = self.sequences.get_mut(idx) {
            sequence.pitch = pitch;
        }
    }

    pub fn set_range_start(&mut self, idx: usize, start: u8) {
        if let Some(sequence) = self.sequences.get_mut(idx) {
            sequence.set_range_start(start);
        }
    }

    pub fn set_range_end(&mut self, idx: usize, end: u8) {
        if let Some(sequence) = self.sequences.get_mut(idx) {
            sequence.set_range_end(end);
        }
    }

    pub fn set_play_mode(&mut self, idx: usize, playmode: PlayMode) {
        if let Some(sequence) = self.sequences.get_mut(idx) {
            sequence.play_mode = playmode;
        }
    }

    pub fn set_attack(&mut self, val: f32) {
        for step in self.steps.iter_mut() {
            step.set_attack(val)
        }
    }

    pub fn set_gain(&mut self, val: f32, sequence: usize) {
        if let Some(sequence) = self.sequences.get_mut(sequence) {
            sequence.gain = val
        }
    }

    pub fn set_release(&mut self, val: f32) {
        for step in self.steps.iter_mut() {
            step.set_release(val)
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
