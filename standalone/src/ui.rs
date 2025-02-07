use std::{io, time::Duration};

use crossbeam::channel::Sender;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};
use sequence_core::{DrawData, Output, Subdivision, STEP_NUM};
use symbols::{STEP_ACTIVE, STEP_INACTIVE};

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
                Constraint::Length(4),
                Constraint::Min(0),
            ])
            .split(frame.area());

        let layout_horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(0),
                Constraint::Min(0),
                Constraint::Min(0),
            ])
            .split(layout_vertical[1]);

        let positions = &draw_data.positions;
        let constraints = vec![Constraint::Length(1); positions.len() + 1];

        let sequences = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(layout_horizontal[1]);

        let spans = Text::from(Line::from(vec![
            Span::from(format!("BPM: {}", draw_data.bpm)),
            Span::from(format!(
                "  {}:{}:{}",
                draw_data.transporter.0, draw_data.transporter.1, draw_data.transporter.2
            )),
        ]));

        frame.render_widget(spans, sequences[0]);

        for (i, position) in positions.iter().enumerate() {
            let mut spans =
                vec![Span::styled(STEP_INACTIVE, Style::default().bold()); STEP_NUM as usize];
            spans[*position as usize] = Span::styled(STEP_ACTIVE, Style::default().bold());
            if self.state.selected_sequence == i {
                spans.push(
                    Span::from(format!("  {}", draw_data.subdivisions[i].get_symbol()))
                        .style(Style::default().fg(Color::Red)),
                );
            } else {
                spans.push(Span::from(format!(
                    "  {}",
                    draw_data.subdivisions[i].get_symbol()
                )));
            }
            let seq_spans = Text::from(Line::from(spans));
            let seq_paragraph = Paragraph::new(seq_spans);
            frame.render_widget(seq_paragraph, sequences[i + 1]);
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
                        _ => {}
                    };
                }
                _ => {}
            }
        }
        Ok(())
    }
}
