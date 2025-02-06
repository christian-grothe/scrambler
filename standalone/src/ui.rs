use std::{io, time::Duration};

use crossbeam::channel::Sender;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};
use sequence_core::{DrawData, Output, STEP_NUM};
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
            },
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let draw_data = self.state.draw_data.read();
        let layout_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Min(0),
                Constraint::Length(4 * 7),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(frame.area());

        let layout_horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(0),
                Constraint::Length(100),
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
            let seq_spans = Text::from(Line::from(spans));
            let seq_paragraph = Paragraph::new(seq_spans);
            frame.render_widget(seq_paragraph, sequences[i + 1]);
        }
    }
}

pub struct State {
    pub exiting: bool,
    draw_data: Output<DrawData>,
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
                        _ => {}
                    };
                }
                _ => {}
            }
        }
        Ok(())
    }
}
