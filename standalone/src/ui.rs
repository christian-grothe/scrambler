use std::{io, time::Duration};

use crossbeam::channel::Sender;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use scrambler_core::{DrawData, Output, STEP_NUM};
use symbols::{
    BLANK, FULL, RANGE_END, RANGE_SINGLE, RANGE_START, SELECTED, STEP_ACTIVE, STEP_INACTIVE,
};

use crate::SetEvent;

mod symbols;

pub struct Ui {
    pub state: State,
}

impl Ui {
    pub fn new(draw_data: Output<DrawData>, sender: Sender<SetEvent>) -> Self {
        Ui {
            state: State {
                exiting: false,
                draw_data,
                sender,
                mode: Mode::Record,
                selected: Selected::Div,
                selected_area: SelectedArea::Sequence(0),
                selected_global: SelectedGlobal::Bpm,
                semitones: vec![0; 3],
                attack: 0.02,
                release: 0.99,
            },
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let draw_data = self.state.draw_data.read();
        let layout_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Min(0),
                Constraint::Min(0),
                Constraint::Min(0),
            ])
            .split(frame.area());

        let layout_horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(0),
                Constraint::Length(50),
                Constraint::Min(0),
            ])
            .split(layout_vertical[1]);

        let positions = &draw_data.positions;

        let main_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(1), // transporter
                Constraint::Length(2), // status
                Constraint::Length(2), // steps
                Constraint::Min(0),    // steps
            ])
            .split(layout_horizontal[1]);

        let sequences = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(5); positions.len()])
            .split(main_area[3]);

        let mode = match self.state.mode {
            Mode::Record => "rec",
            Mode::Erase => "erase",
            Mode::RangeStart => "range start",
            Mode::RangeEnd => "range end",
        };

        let mut status_spans = vec![
            Span::from(format!(" BPM: {} ", draw_data.bpm)),
            Span::from(format!("   att: {:.2} ", self.state.attack)),
            Span::from(format!("   rel: {:.2} ", self.state.release)),
        ];

        if self.state.selected_area == SelectedArea::Global {
            match self.state.selected_global {
                SelectedGlobal::Bpm => {
                    status_spans[0] = status_spans[0]
                        .clone()
                        .style(Style::default().fg(Color::Red))
                }
                SelectedGlobal::Att => {
                    status_spans[1] = status_spans[1]
                        .clone()
                        .style(Style::default().fg(Color::Red))
                }
                SelectedGlobal::Rel => {
                    status_spans[2] = status_spans[2]
                        .clone()
                        .style(Style::default().fg(Color::Red))
                }
            }
        }

        let status_bar = Paragraph::new(Text::from(Line::from(status_spans)))
            .block(Block::new().borders(Borders::BOTTOM));

        frame.render_widget(status_bar, main_area[1]);

        let transporter_span = vec![
            Span::from(format!(
                " {}:{}   |",
                draw_data.transporter.1 + 1,
                draw_data.transporter.2 + 1
            )),
            Span::from(format!("   Mode: {}", mode)),
        ];
        frame.render_widget(
            Paragraph::new(Text::from(Line::from(transporter_span))),
            main_area[0],
        );

        let step_status_span: Vec<Span> = draw_data
            .step_states
            .iter()
            .map(|state| Span::from(state.get_symbol()))
            .collect();

        frame.render_widget(
            Paragraph::new(Text::from(Line::from(step_status_span))),
            main_area[2],
        );

        for (i, position) in positions.iter().enumerate() {
            let sequence_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(1); 3])
                .split(sequences[i]);

            // render steps
            let mut steps =
                vec![Span::styled(STEP_INACTIVE, Style::default().bold()); STEP_NUM as usize];

            steps[*position as usize] = Span::styled(STEP_ACTIVE, Style::default().bold());

            if self.state.selected_area == SelectedArea::Sequence(i) {
                steps.push(Span::from(SELECTED).style(Style::default().fg(Color::Red)));
            }

            frame.render_widget(
                Paragraph::new(Text::from(Line::from(steps))),
                sequence_area[0],
            );

            // render range
            let mut range = vec![Span::styled(BLANK, Style::default().bold()); STEP_NUM as usize];
            for y in draw_data.ranges[i].0..draw_data.ranges[i].1 {
                range[y as usize] = Span::styled(FULL, Style::default().bold());
            }
            if draw_data.ranges[i].0 == draw_data.ranges[i].1 {
                range[draw_data.ranges[i].0 as usize] =
                    Span::styled(RANGE_SINGLE, Style::default().bold());
            } else {
                range[draw_data.ranges[i].0 as usize] =
                    Span::styled(RANGE_START, Style::default().bold());
                range[draw_data.ranges[i].1 as usize] =
                    Span::styled(RANGE_END, Style::default().bold());
            }

            frame.render_widget(
                Paragraph::new(Text::from(Line::from(range))),
                sequence_area[1],
            );

            // render params
            let mut param_lines = vec![
                Span::from(format!(" Div: {} ", draw_data.subdivisions[i].get_symbol())),
                Span::from(format!(" Dir: {} ", draw_data.dirs[i].get_symbol())),
                Span::from(format!(" Pitch: {} ", self.state.semitones[i])),
                Span::from(format!(" Gain: {:.1} ", draw_data.gains[i])),
            ];

            if self.state.selected_area == SelectedArea::Sequence(i) {
                match self.state.selected {
                    Selected::Div => {
                        param_lines[0] = param_lines[0]
                            .clone()
                            .style(Style::default().fg(Color::Red))
                    }
                    Selected::Dir => {
                        param_lines[1] = param_lines[1]
                            .clone()
                            .style(Style::default().fg(Color::Red))
                    }
                    Selected::Pitch => {
                        param_lines[2] = param_lines[2]
                            .clone()
                            .style(Style::default().fg(Color::Red))
                    }
                    Selected::Gain => {
                        param_lines[3] = param_lines[3]
                            .clone()
                            .style(Style::default().fg(Color::Red))
                    }
                };
            };

            let params = Paragraph::new(Text::from(Line::from(param_lines)));

            frame.render_widget(params, sequence_area[2]);
        }
    }
}

enum Mode {
    Record,
    Erase,
    RangeStart,
    RangeEnd,
}

#[derive(PartialEq)]
enum Selected {
    Div,
    Dir,
    Pitch,
    Gain,
}

#[derive(PartialEq)]
enum SelectedGlobal {
    Bpm,
    Att,
    Rel,
}

impl SelectedGlobal {
    fn next(&mut self) {
        *self = match self {
            SelectedGlobal::Bpm => SelectedGlobal::Att,
            SelectedGlobal::Att => SelectedGlobal::Rel,
            SelectedGlobal::Rel => SelectedGlobal::Att,
        }
    }

    fn prev(&mut self) {
        *self = match self {
            SelectedGlobal::Bpm => SelectedGlobal::Rel,
            SelectedGlobal::Att => SelectedGlobal::Bpm,
            SelectedGlobal::Rel => SelectedGlobal::Att,
        }
    }
}

#[derive(PartialEq)]
enum SelectedArea {
    Sequence(usize),
    Global,
}

impl SelectedArea {
    fn next(&mut self) {
        *self = match self {
            SelectedArea::Sequence(idx) => {
                if *idx >= 2 {
                    SelectedArea::Global
                } else {
                    SelectedArea::Sequence(*idx + 1)
                }
            }
            SelectedArea::Global => SelectedArea::Sequence(0),
        }
    }

    fn prev(&mut self) {
        *self = match self {
            SelectedArea::Sequence(idx) => {
                if *idx <= 0 {
                    SelectedArea::Global
                } else {
                    SelectedArea::Sequence(*idx - 1)
                }
            }
            SelectedArea::Global => SelectedArea::Sequence(2),
        }
    }
}

impl Selected {
    fn next(&mut self) {
        *self = match self {
            Selected::Div => Selected::Dir,
            Selected::Dir => Selected::Pitch,
            Selected::Pitch => Selected::Gain,
            Selected::Gain => Selected::Div,
        };
    }

    fn prev(&mut self) {
        *self = match self {
            Selected::Div => Selected::Gain,
            Selected::Gain => Selected::Dir,
            Selected::Dir => Selected::Div,
            Selected::Pitch => Selected::Div,
        };
    }
}

pub struct State {
    pub exiting: bool,
    draw_data: Output<DrawData>,
    selected_area: SelectedArea,
    selected_global: SelectedGlobal,
    mode: Mode,
    sender: Sender<SetEvent>,
    selected: Selected,
    semitones: Vec<i8>,
    attack: f32,
    release: f32,
}

impl State {
    pub fn handle_event(&mut self, ms: u64) -> io::Result<()> {
        let draw_data = self.draw_data.read();
        if event::poll(Duration::from_millis(ms))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match key_event.code {
                        KeyCode::Esc => self.exiting = true,
                        KeyCode::Char('m') => match self.mode {
                            Mode::Record => self.mode = Mode::Erase,
                            Mode::Erase => self.mode = Mode::RangeStart,
                            Mode::RangeStart => self.mode = Mode::RangeEnd,
                            Mode::RangeEnd => self.mode = Mode::Record,
                        },
                        KeyCode::Char('j') => {
                            self.selected_area.next();
                        }
                        KeyCode::Char('k') => {
                            self.selected_area.prev();
                        }
                        KeyCode::Char('l') => match self.selected_area {
                            SelectedArea::Sequence(_) => self.selected.next(),
                            SelectedArea::Global => self.selected_global.next(),
                        },
                        KeyCode::Char('h') => match self.selected_area {
                            SelectedArea::Sequence(_) => self.selected.prev(),
                            SelectedArea::Global => self.selected_global.prev(),
                        },
                        KeyCode::Char('K') => match self.selected_area {
                            SelectedArea::Sequence(idx) => match self.selected {
                                Selected::Pitch => {
                                    let semitone = &mut self.semitones[idx];
                                    if *semitone >= 12 {
                                        *semitone = -12;
                                    } else {
                                        *semitone += 1;
                                    }
                                    self.sender.send(SetEvent::SetPitch((idx, *semitone)))
                                }
                                .unwrap(),
                                Selected::Div => self
                                    .sender
                                    .send(SetEvent::SetSubdivision((
                                        idx,
                                        draw_data.subdivisions[idx].next(),
                                    )))
                                    .unwrap(),
                                Selected::Dir => self
                                    .sender
                                    .send(SetEvent::SetDir((idx, draw_data.dirs[idx].next())))
                                    .unwrap(),
                                Selected::Gain => self
                                    .sender
                                    .send(SetEvent::SetGain((idx, draw_data.gains[idx] + 0.1)))
                                    .unwrap(),
                            },
                            SelectedArea::Global => match self.selected_global {
                                SelectedGlobal::Bpm => self
                                    .sender
                                    .send(SetEvent::SetBmp(draw_data.bpm + 1.0))
                                    .unwrap(),
                                SelectedGlobal::Att => {
                                    let att = &mut self.attack;
                                    *att += 0.01;
                                    if *att >= 0.99 {
                                        *att = 0.99;
                                    }
                                    self.sender.send(SetEvent::SetAttack(*att)).unwrap();
                                }
                                SelectedGlobal::Rel => {
                                    let rel = &mut self.release;
                                    *rel += 0.01;
                                    if *rel >= 0.99 {
                                        *rel = 0.99
                                    }
                                    self.sender.send(SetEvent::SetRelease(*rel)).unwrap();
                                }
                            },
                        },
                        KeyCode::Char('J') => match self.selected_area {
                            SelectedArea::Sequence(idx) => match self.selected {
                                Selected::Pitch => {
                                    let semitone = &mut self.semitones[idx];
                                    if *semitone <= -12 {
                                        *semitone = 12;
                                    } else {
                                        *semitone -= 1;
                                    }
                                    self.sender.send(SetEvent::SetPitch((idx, *semitone)))
                                }
                                .unwrap(),
                                Selected::Div => self
                                    .sender
                                    .send(SetEvent::SetSubdivision((
                                        idx,
                                        draw_data.subdivisions[idx].prev(),
                                    )))
                                    .unwrap(),
                                Selected::Dir => self
                                    .sender
                                    .send(SetEvent::SetDir((idx, draw_data.dirs[idx].prev())))
                                    .unwrap(),
                                Selected::Gain => self
                                    .sender
                                    .send(SetEvent::SetGain((idx, draw_data.gains[idx] - 0.1)))
                                    .unwrap(),
                            },
                            SelectedArea::Global => match self.selected_global {
                                SelectedGlobal::Bpm => self
                                    .sender
                                    .send(SetEvent::SetBmp(draw_data.bpm - 1.0))
                                    .unwrap(),
                                SelectedGlobal::Att => {
                                    let att = &mut self.attack;
                                    *att -= 0.01;
                                    if *att <= 0.02 {
                                        *att = 0.02
                                    }
                                    self.sender.send(SetEvent::SetAttack(*att)).unwrap();
                                }
                                SelectedGlobal::Rel => {
                                    let rel = &mut self.release;
                                    *rel -= 0.01;
                                    if *rel <= 0.02 {
                                        *rel = 0.02
                                    }
                                    self.sender.send(SetEvent::SetRelease(*rel)).unwrap();
                                }
                            },
                        },
                        KeyCode::Char('1') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(0)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(0)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 0))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 0))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char('2') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(1)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(1)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 1))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 1))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char('3') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(2)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(2)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 2))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 2))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char('4') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(3)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(3)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 3))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 3))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char('5') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(4)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(4)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 4))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 4))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char('6') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(5)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(5)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 5))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 5))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char('7') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(6)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(6)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 6))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 6))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char('8') => match self.mode {
                            Mode::Record => self.sender.send(SetEvent::Record(7)).unwrap(),
                            Mode::Erase => self.sender.send(SetEvent::Erase(7)).unwrap(),
                            Mode::RangeStart => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeStart((idx, 7))).unwrap()
                                }
                                _ => {}
                            },
                            Mode::RangeEnd => match self.selected_area {
                                SelectedArea::Sequence(idx) => {
                                    self.sender.send(SetEvent::SetRangeEnd((idx, 7))).unwrap()
                                }
                                _ => {}
                            },
                        },
                        KeyCode::Char(' ') => match self.selected_area {
                            SelectedArea::Sequence(idx) => {
                                self.sender.send(SetEvent::Toggle(idx)).unwrap()
                            }
                            _ => {}
                        },
                        _ => {}
                    };
                }
                _ => {}
            }
        }
        Ok(())
    }
}
