pub mod canvas;
pub mod model;
pub mod terminal;

use std::cmp;
use std::io;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time;

use termion;
use termion::clear;
use termion::color::*;
use termion::event::Key::*;
use termion::input::TermRead;

use crate::config::CHOOSER_HEIGHT;

use crate::core;

fn print_state(term_canvas: &mut canvas::TermCanvas, model: &model::Model) -> io::Result<()> {
    let off = model.view_offset();
    let prompt = "  Search: ";
    let count = model.completions_count();
    let status_string = format!(
        "[{} {}-{}/{}]",
        model.completer_name(),
        off + 1,
        cmp::min(off + CHOOSER_HEIGHT + 1, count),
        count,
    );

    term_canvas.clear()?;
    write!(term_canvas, "{}{}", prompt, model.query())?;
    let term_width = term_canvas.width();
    term_canvas.move_to(0, term_width - status_string.len())?;
    write!(term_canvas, "{}", status_string)?;

    let end_offset = cmp::min(off + CHOOSER_HEIGHT, count);
    for i in off..end_offset {
        let (comp, score) = model.completion_at(i);
        let completion_string = comp.display_string();
        let displayed_length = cmp::min(completion_string.len(), term_canvas.width() - 2);
        let displayed_completion = &(completion_string)[..displayed_length];
        term_canvas.move_to(i + 1, 0)?;
        if off + i == model.selection() {
            write!(
                term_canvas,
                "{}{}{} {}{}{}",
                Bg(Black),
                Fg(White),
                score,
                displayed_completion,
                Fg(Reset),
                Bg(Reset)
            )?;
        } else {
            write!(term_canvas, "{} {}", score, displayed_completion)?;
        }
    }

    term_canvas.move_to(0, prompt.len() + model.query().len())?;

    return Result::Ok(());
}

fn key_reader_thread_routine(
    req_receiver: mpsc::Receiver<()>,
    key_sender: mpsc::Sender<termion::event::Key>,
) {
    let mut keys = io::stdin().keys();
    while let Result::Ok(()) = req_receiver.recv() {
        if let Some(Result::Ok(key)) = keys.next() {
            let result = key_sender.send(key);
            if result.is_err() {
                break;
            }
        } else {
            break;
        }
    }
}

pub fn get_completion(
    initial_query: &str,
    completers: Vec<Box<dyn core::Completer>>,
) -> io::Result<String> {
    let term = termion::get_tty()?;
    let mut model = model::Model::new(completers);

    model.query_set(&initial_query);

    let original_terminal_state = terminal::prepare()?;

    let mut term_canvas = canvas::TermCanvas::new(term, CHOOSER_HEIGHT + 1)?;

    model.start_fetching_completions();

    let result: String;

    let (key_sender, key_receiver) = mpsc::channel::<termion::event::Key>();
    let (req_sender, req_receiver) = mpsc::channel::<()>();
    let key_reader_thread =
        thread::spawn(move || key_reader_thread_routine(req_receiver, key_sender));
    let mut req_sender = Some(req_sender);

    req_sender.as_ref().unwrap().send(()).unwrap();
    loop {
        print_state(&mut term_canvas, &model)?;

        let key_or_nothing;
        if !model.fetching_completions_finished() {
            key_or_nothing = key_receiver
                .recv_timeout(time::Duration::from_millis(10))
                .ok();
            model.fetch_completions();
        } else {
            key_or_nothing = key_receiver.recv().ok();
        }

        if let Some(key) = key_or_nothing {
            match key {
                Up => model.select_previous(),
                Down => model.select_next(),
                PageUp => model.previous_page(),
                PageDown => model.next_page(),
                Home => model.select_first(),
                End => model.select_last(),

                Left => model.ascend(),
                Right => model.descend(),

                Char('\n') => {
                    if let Some(r) = model.get_selected_result() {
                        result = r;
                        break;
                    }
                }
                Ctrl('c') => {
                    result = initial_query.to_owned();
                    break;
                }
                Char('\t') => model.next_tab(),
                Char(c) => model.query_append(c),
                Backspace => model.query_backspace(),

                _ => {}
            };
            req_sender.as_ref().unwrap().send(()).unwrap();
        }
    }

    req_sender.take();
    key_reader_thread.join().unwrap();

    clear()?;
    terminal::restore(original_terminal_state)?;

    return Result::Ok(result);
}

pub fn clear() -> io::Result<()> {
    let mut term = termion::get_tty()?;
    for _ in 0..(CHOOSER_HEIGHT + 1) {
        write!(term, "{}{}", clear::CurrentLine, termion::cursor::Down(1))?;
    }
    write!(
        term,
        "{}{}",
        termion::cursor::Left(100),
        termion::cursor::Up((CHOOSER_HEIGHT + 1) as u16)
    )?;
    return Result::Ok(());
}
