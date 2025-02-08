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
use sequence_core::{DrawData, Output, Subdivision, STEP_NUM};
use symbols::{BLANK, FULL, RANGE_END, RANGE_START, STEP_ACTIVE, STEP_INACTIVE};

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
                selected_sequence: 0,
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
                Constraint::Length(2), // status_bar
                Constraint::Min(0),    // steps
                Constraint::Min(0),    // params
            ])
            .split(layout_horizontal[1]);

        let sequences = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3); positions.len()])
            .split(main_area[1]);

        let status_bar = Paragraph::new(Text::from(Line::from(vec![
            Span::from(format!("BPM: {}", draw_data.bpm)),
            Span::from(format!(
                "  {}:{}:{}",
                draw_data.transporter.0, draw_data.transporter.1, draw_data.transporter.2
            )),
        ])))
        .block(Block::new().borders(Borders::BOTTOM));

        frame.render_widget(status_bar, main_area[0]);

        for (i, position) in positions.iter().enumerate() {
            let sequence_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(1); 2])
                .split(sequences[i]);

            // render steps
            let mut steps =
                vec![Span::styled(STEP_INACTIVE, Style::default().bold()); STEP_NUM as usize];

            steps[*position as usize] = Span::styled(STEP_ACTIVE, Style::default().bold());

            if self.state.selected_sequence == i {
                steps.push(
                    Span::from(format!("  {}", draw_data.subdivisions[i].get_symbol()))
                        .style(Style::default().fg(Color::Red)),
                );
            } else {
                steps.push(Span::from(format!(
                    "  {}",
                    draw_data.subdivisions[i].get_symbol()
                )));
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
            range[draw_data.ranges[i].0 as usize] =
                Span::styled(RANGE_START, Style::default().bold());
            range[draw_data.ranges[i].1 as usize] =
                Span::styled(RANGE_END, Style::default().bold());

            frame.render_widget(
                Paragraph::new(Text::from(Line::from(range))),
                sequence_area[1],
            );
        }
    }
}

pub struct State {
    pub exiting: bool,
    draw_data: Output<DrawData>,
    selected_sequence: usize,
    sender: Sender<SetEvent>,
}

impl State {
    pub fn handle_event(&mut self, ms: u64) -> io::Result<()> {
        let draw_data = self.draw_data.read();
        if event::poll(Duration::from_millis(ms))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match key_event.code {
                        KeyCode::Esc => self.exiting = true,
                        KeyCode::Char('k') => self
                            .sender
                            .send(SetEvent::SetBmp(draw_data.bpm + 1.0))
                            .unwrap(),
                        KeyCode::Char('j') => self
                            .sender
                            .send(SetEvent::SetBmp(draw_data.bpm - 1.0))
                            .unwrap(),
                        KeyCode::Char('l') => self
                            .sender
                            .send(SetEvent::SetPitch((
                                self.selected_sequence,
                                draw_data.pitches[self.selected_sequence] + 0.1,
                            )))
                            .unwrap(),
                        KeyCode::Char('h') => self
                            .sender
                            .send(SetEvent::SetPitch((
                                self.selected_sequence,
                                draw_data.pitches[self.selected_sequence] - 0.1,
                            )))
                            .unwrap(),
                        KeyCode::Char('1') => self.sender.send(SetEvent::Record(0)).unwrap(),
                        KeyCode::Char('2') => self.sender.send(SetEvent::Record(1)).unwrap(),
                        KeyCode::Char('3') => self.sender.send(SetEvent::Record(2)).unwrap(),
                        KeyCode::Char('4') => self.sender.send(SetEvent::Record(3)).unwrap(),
                        KeyCode::Char('5') => self.sender.send(SetEvent::Record(4)).unwrap(),
                        KeyCode::Char('6') => self.sender.send(SetEvent::Record(5)).unwrap(),
                        KeyCode::Char('7') => self.sender.send(SetEvent::Record(6)).unwrap(),
                        KeyCode::Char('8') => self.sender.send(SetEvent::Record(7)).unwrap(),
                        KeyCode::Char('a') => self.selected_sequence = 0,
                        KeyCode::Char('s') => self.selected_sequence = 1,
                        KeyCode::Char('d') => self.selected_sequence = 2,
                        KeyCode::Char('q') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::Quarter,
                            )))
                            .unwrap(),
                        KeyCode::Char('w') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::Eighth,
                            )))
                            .unwrap(),
                        KeyCode::Char('e') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::Sixteenth,
                            )))
                            .unwrap(),
                        KeyCode::Char('r') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::ThirtySecond,
                            )))
                            .unwrap(),
                        KeyCode::Char('t') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::TripletQuarter,
                            )))
                            .unwrap(),
                        KeyCode::Char('z') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::TripletEighth,
                            )))
                            .unwrap(),
                        KeyCode::Char('u') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::TripletSixteenth,
                            )))
                            .unwrap(),
                        KeyCode::Char('i') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::DottedQuarter,
                            )))
                            .unwrap(),
                        KeyCode::Char('o') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::DottedEighth,
                            )))
                            .unwrap(),
                        KeyCode::Char('p') => self
                            .sender
                            .send(SetEvent::SetSubdivision((
                                self.selected_sequence,
                                Subdivision::DottedSixteenth,
                            )))
                            .unwrap(),
                        KeyCode::Enter => self
                            .sender
                            .send(SetEvent::Toggle(self.selected_sequence))
                            .unwrap(),
                        _ => {}
                    };
                }
                _ => {}
            }
        }
        Ok(())
    }
}
