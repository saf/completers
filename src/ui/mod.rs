pub mod model;
pub mod terminal;

use std::cmp;
use std::fs::File;
use std::io;
use std::io::Write;
use std::sync::mpsc;
use std::time;
use std::thread;

use termion;
use termion::clear;
use termion::input::TermRead;
use termion::color::*;
use termion::event::Key::*;

use config::*;

use core;

fn print_state(term: &mut File, model: &model::Model) -> io::Result<()> {
    let off = model.view_offset();
    let prompt = "  Search: ";
    let completions = model.completions();
    let status_string = format!("[{} {}-{}/{}]", model.completer_name(), off + 1,
                                cmp::min(off + CHOOSER_HEIGHT + 1, completions.len()),
                                completions.len());
    let term_cols = terminal::get_width(term).unwrap() as usize;

    writeln!(term, "{}{}{}{}{:>sw$}", termion::cursor::Left(100),
             clear::CurrentLine, prompt, model.query(), status_string,
             sw = term_cols - prompt.len() - model.query().len())?;

    let end_offset = cmp::min(off + CHOOSER_HEIGHT, completions.len());
    for (i, p) in completions[off .. end_offset].iter().enumerate() {
        let completion_string = p.display_string();
        let displayed_length = cmp::min(completion_string.len(), term_cols - 2);
        let displayed_completion = &(completion_string)[..displayed_length];
        if off + i == model.selection() {
            writeln!(term, "{}{}{}{}{}{}",
                     clear::CurrentLine, Bg(Black), Fg(White),
                     displayed_completion,
                     Fg(Reset), Bg(Reset))?;
        } else {
            writeln!(term, "{}{}", clear::CurrentLine, displayed_completion)?;
        }
    }

    for _ in end_offset .. off + CHOOSER_HEIGHT {
        writeln!(term, "{}", clear::CurrentLine)?;
    }

    write!(term, "{}{}",
           termion::cursor::Up((CHOOSER_HEIGHT + 1) as u16),
           termion::cursor::Right((prompt.len() + model.query().len()) as u16))?;
    return Result::Ok(());
}

pub fn get_initial_query(line: &str) -> String {
    let line_length = line.len();
    let last_word_boundary = line.rfind(WORD_BOUNDARIES);
    let word_index = match last_word_boundary {
        None => 0,
        Some(index) => index + 1,
    };

    if word_index >= line_length {
        "".to_string()
    } else {
        line[word_index..].to_string()
    }
}

fn key_reader_thread_routine(req_receiver: mpsc::Receiver<()>,
                             key_sender: mpsc::Sender<termion::event::Key>) {
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

pub fn get_completion(mut line: String, completers: Vec<Box<core::Completer>>)
                      -> io::Result<(String, i16)> {
    let mut term = termion::get_tty()?;
    let mut model = model::Model::new(completers);

    let original_query = get_initial_query(line.as_str());
    model.query_set(original_query.as_str());

    let original_terminal_state = terminal::prepare()?;
    write!(term, "{}", termion::cursor::Right(30))?;

    model.start_fetching_completions();
    print_state(&mut term, &model).unwrap();

    let result;

    let (key_sender, key_receiver) = mpsc::channel::<termion::event::Key>();
    let (req_sender, req_receiver) = mpsc::channel::<()>();
    let key_reader_thread = thread::spawn(move || key_reader_thread_routine(req_receiver, key_sender));
    let mut req_sender = Some(req_sender);

    req_sender.as_ref().unwrap().send(()).unwrap();
    loop {
        let key_or_nothing;
        if !model.fetching_completions_finished() {
            key_or_nothing = key_receiver.recv_timeout(time::Duration::from_millis(10)).ok();
            model.fetch_completions();
        } else {
            key_or_nothing = key_receiver.recv().ok();
        }

        if let Some(key) = key_or_nothing {
            match key {
                Up         => model.select_previous(),
                Down       => model.select_next(),
                PageUp     => model.previous_page(),
                PageDown   => model.next_page(),
                Home       => model.select_first(),
                End        => model.select_last(),

                Left       => model.ascend(),
                Right      => model.descend(),

                Char('\n') => {
                    if let Some(r) = model.get_selected_result() {
                        result = r;
                        break
                    }
                },
                Ctrl('c')  => { result = original_query.clone(); break },
                Char('\t') => model.next_tab(),
                Char(c)    => model.query_append(c),
                Backspace  => model.query_backspace(),

                _ => {},
            };
            // We are going to loop again, so we send a request to get another input key.
            req_sender.as_ref().unwrap().send(()).unwrap();
        }
        print_state(&mut term, &model)?;
    }

    req_sender.take();
    key_reader_thread.join().unwrap();

    clear()?;
    terminal::restore(original_terminal_state)?;

    let line_length = line.len();
    let original_length = original_query.len();
    let new_length = result.len();
    line.truncate(line_length - original_length);
    line.push_str(&result);
    return Result::Ok((line, (new_length - original_length) as i16));
}

pub fn clear() -> io::Result<()> {
    let mut term = termion::get_tty()?;
    for _ in 0..(CHOOSER_HEIGHT + 1) {
        write!(term, "{}{}", clear::CurrentLine, termion::cursor::Down(1))?;
    }
    write!(term, "{}{}", termion::cursor::Left(100),
           termion::cursor::Up((CHOOSER_HEIGHT + 1) as u16))?;
    return Result::Ok(());
}
