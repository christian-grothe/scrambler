#[derive(PartialEq)]
enum EnvState {
    Attack,
    Release,
}

pub struct Voice {
    play_head: f32,
    pub is_playing: bool,
    pub gain: f32,
    env: f32,
    env_inc_attack: f32,
    env_inc_release: f32,
    env_state: EnvState,
    buffer_size: usize,
    pub pitch: f32,
    sample_rate: f32,
}

impl Voice {
    pub fn new(sample_rate: f32, buffer_size: usize) -> Self {
        Voice {
            play_head: 0.0,
            is_playing: false,
            gain: 0.0,
            env: 0.0,
            env_inc_attack: 1.0 / sample_rate / 0.01,
            env_inc_release: 1.0 / sample_rate / 1.0,
            env_state: EnvState::Attack,
            buffer_size,
            pitch: 1.0,
            sample_rate,
        }
    }

    pub fn render(&mut self) -> (f32, f32, f32) {
        match self.env_state {
            EnvState::Attack => self.env += self.env_inc_attack,
            EnvState::Release => self.env -= self.env_inc_release,
        };

        if self.env >= 1.0 {
            self.env_state = EnvState::Release;
        }

        if self.env <= 0.0 && self.env_state == EnvState::Release {
            self.env = 0.0;
        }

        self.play_head += self.pitch;
        if self.play_head >= self.buffer_size as f32 {
            self.is_playing = false;
            self.play_head = 0.0;
            self.env_state = EnvState::Attack;
        }

        (self.play_head, self.env, self.gain)
    }

    pub fn set_attack(&mut self, val: f32) {
        self.env_inc_attack = 1.0 / self.sample_rate / (val / self.pitch);
    }

    pub fn set_release(&mut self, val: f32) {
        self.env_inc_release = 1.0 / self.sample_rate / (val / self.pitch);
    }
}
