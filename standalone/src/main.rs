use std::{
    io::{self, stdout},
    time::{Duration, Instant},
};

use crossbeam::channel::{unbounded, Receiver};
use jack::{AudioIn, AudioOut, Client, ClientOptions, Port};
use ratatui::crossterm::{
    event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
};

use scrambler_core::*;

mod ui;

enum SetEvent {
    SetBmp(f32),
    Record(usize),
    Erase(usize),
    SetSubdivision((usize, Subdivision)),
    Toggle(usize),
    SetPitch((usize, i8)),
    SetRangeStart((usize, u8)),
    SetRangeEnd((usize, u8)),
    SetDir((usize, PlayMode)),
    SetAttack((usize, f32)),
    SetRelease((usize, f32)),
    SetGain((usize, f32)),
}

fn main() -> io::Result<()> {
    let (s, r) = unbounded();

    let (client, _status) = Client::new("sequencer", ClientOptions::default()).unwrap();

    let out_port_l = client
        .register_port("output_l", AudioOut::default())
        .unwrap();

    // let out_port_r = client
    //     .register_port("output_r", AudioOut::default())
    //     .unwrap();

    let input_port = client.register_port("input", AudioIn::default()).unwrap();

    let (sequencer, draw_data) = scrambler_core::Sequencer::new(48000.0);

    struct State {
        input: Port<AudioIn>,
        output_l: Port<AudioOut>,
        //output_r: Port<AudioOut>,
        sequencer: scrambler_core::Sequencer,
        receiver: Receiver<SetEvent>,
    }

    let process = jack::contrib::ClosureProcessHandler::with_state(
        State {
            sequencer,
            input: input_port,
            output_l: out_port_l,
            //output_r: out_port_r,
            receiver: r,
        },
        |state, _, ps| -> jack::Control {
            let output_l = state.output_l.as_mut_slice(ps);
            //let output_r = state.output_r.as_mut_slice(ps);
            let input = state.input.as_slice(ps);

            output_l.copy_from_slice(input);
            //output_r.copy_from_slice(input);

            let events = state.receiver.try_iter();
            for event in events {
                match event {
                    SetEvent::SetBmp(val) => state.sequencer.set_bpm(val),
                    SetEvent::Record(idx) => state.sequencer.record(idx),
                    SetEvent::Erase(idx) => state.sequencer.erase(idx),
                    SetEvent::SetSubdivision((index, val)) => {
                        state.sequencer.set_subdivision(index, val)
                    }
                    SetEvent::Toggle(index) => state.sequencer.toggle(index),
                    SetEvent::SetPitch((index, pitch)) => state.sequencer.set_pitch(index, pitch),
                    SetEvent::SetRangeStart((index, pitch)) => {
                        state.sequencer.set_range_start(index, pitch)
                    }
                    SetEvent::SetRangeEnd((index, pitch)) => {
                        state.sequencer.set_range_end(index, pitch)
                    }
                    SetEvent::SetDir((index, playmode)) => {
                        state.sequencer.set_play_mode(index, playmode)
                    }
                    SetEvent::SetAttack((index, val)) => state.sequencer.set_attack(val, index),
                    SetEvent::SetRelease((index, val)) => state.sequencer.set_release(val, index),
                    SetEvent::SetGain((index, val)) => state.sequencer.set_gain(val, index),
                }
            }

            for sample in output_l.iter_mut() {
                state.sequencer.render(sample);
            }

            jack::Control::Continue
        },
        move |_, _, _| jack::Control::Continue,
    );

    let _active_client = client.activate_async((), process).unwrap();

    let mut terminal = ratatui::init();
    let mut stdout = stdout();

    execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )?;

    let tick_rate = Duration::from_millis(30);
    let mut last_tick = Instant::now();

    let mut ui_handler = ui::Ui::new(draw_data, s);

    while !ui_handler.state.exiting {
        ui_handler.state.handle_event(1)?;
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();

            terminal.draw(|f| ui_handler.draw(f))?;
        }
    }

    ratatui::restore();

    Ok(())
}
