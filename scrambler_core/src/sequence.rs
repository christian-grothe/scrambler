use crate::{Counter, Subdivision};

#[derive(PartialEq)]
pub enum PlayState {
    Playing,
    Stopped,
    Resume,
}

#[derive(Clone)]
pub enum PlayMode {
    Forwards,
    Backwards,
    BackAndForth(u8),
}

impl PlayMode {
    pub fn get_symbol(&self) -> &str {
        match self {
            PlayMode::Forwards => ">>",
            PlayMode::Backwards => "<<",
            PlayMode::BackAndForth(_) => "<>",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            PlayMode::Forwards => PlayMode::Backwards,
            PlayMode::Backwards => PlayMode::BackAndForth(0),
            PlayMode::BackAndForth(_) => PlayMode::Forwards,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            PlayMode::Forwards => PlayMode::BackAndForth(0),
            PlayMode::Backwards => PlayMode::Forwards,
            PlayMode::BackAndForth(_) => PlayMode::Backwards,
        }
    }
}

pub struct Sequence {
    pub subdivision: Subdivision,
    pub next_subdivision: Option<Subdivision>,
    counter: Counter,
    pub current_step: u8,
    pub pitch: f32,
    pub play_state: PlayState,
    pub play_mode: PlayMode,
    pub play_range: (u8, u8),
    pub gain: f32,
}

impl Sequence {
    pub fn new(
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
            gain: 0.8,
        }
    }

    pub fn set_range_start(&mut self, pos: u8) {
        self.play_range.0 = pos;
        if self.play_range.0 > self.play_range.1 {
            self.play_range.1 = pos;
        }
    }

    pub fn set_range_end(&mut self, pos: u8) {
        self.play_range.1 = pos;
        if self.play_range.1 < self.play_range.0 {
            self.play_range.0 = pos;
        }
    }

    pub fn toggle(&mut self) {
        match self.play_state {
            PlayState::Playing => {
                self.play_state = PlayState::Stopped;
                self.counter.reset();
            }
            PlayState::Stopped => self.play_state = PlayState::Resume,
            PlayState::Resume => self.play_state = PlayState::Stopped,
        }
    }

    pub fn update(&mut self, apply_change: bool, current_bpm: f32) -> Option<(u8, f32, f32)> {
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
                    if self.current_step >= self.play_range.1 {
                        self.current_step = self.play_range.0;
                    } else {
                        self.current_step += 1;
                    }
                }
                PlayMode::Backwards => {
                    if self.current_step <= self.play_range.0 {
                        self.current_step = self.play_range.1;
                    } else {
                        self.current_step -= 1;
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
            Some((self.current_step, self.pitch, self.gain))
        }
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.counter.set_frequency(self.subdivision.to_hz(bpm));
    }

    pub fn apply_subdivision(&mut self, current_bpm: f32) {
        if let Some(subdivision) = self.next_subdivision {
            self.subdivision = subdivision;
            self.set_bpm(current_bpm);
            self.counter.reset();
            self.next_subdivision = None;
        }
    }

    pub fn set_subdivision(&mut self, subdivision: Subdivision) {
        self.next_subdivision = Some(subdivision);
    }
}
