pub mod canvas;
pub mod model;
pub mod terminal;

use std::cmp;
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

fn print_state(term_canvas: &mut canvas::TermCanvas, model: &model::Model) -> io::Result<()> {
    let off = model.view_offset();
    let prompt = "  Search: ";
    let completions = model.completions();
    let status_string = format!("[{} {}-{}/{}]", model.completer_name(), off + 1,
                                cmp::min(off + CHOOSER_HEIGHT + 1, completions.len()),
                                completions.len());

    term_canvas.clear()?;
    write!(term_canvas, "{}{}", prompt, model.query())?;
    let term_width = term_canvas.width();
    term_canvas.move_to(0, term_width - status_string.len())?;
    write!(term_canvas, "{}", status_string)?;

    let end_offset = cmp::min(off + CHOOSER_HEIGHT, completions.len());
    for (i, p) in completions[off .. end_offset].iter().enumerate() {
        let completion_string = p.display_string();
        let displayed_length = cmp::min(completion_string.len(), term_canvas.width() - 2);
        let displayed_completion = &(completion_string)[..displayed_length];
        term_canvas.move_to(i + 1, 0)?;
        if off + i == model.selection() {
            write!(term_canvas, "{}{}{}{}{}",
                     Bg(Black), Fg(White),
                     displayed_completion,
                     Fg(Reset), Bg(Reset))?;
        } else {
            write!(term_canvas, "{}", displayed_completion)?;
        }
    }

    term_canvas.move_to(0, prompt.len() + model.query().len())?;

    return Result::Ok(());
}

/// Returns a pair of character indices within `line`
/// which delimit the initial query, i.e., the string
/// which will be substituted by completions.
///
/// This returns a pair representing the range [start, end).
fn get_initial_query_range(line: &str, point: usize) -> (usize, usize) {
    let words = line.split(WORD_BOUNDARIES);
    let mut start : usize = 0;
    for w in words {
        let end = start + w.len();
        if point >= start && point <= end {
            return (start, end);
        }
        // Moving forward, we have to add 1 for the delimiter itself.
        start = end + 1;
    }
    // If we get here, it means that there were no words.
    (0, 0)
}

#[test]
fn test_initial_query_range() {
    assert_eq!((0, 0), get_initial_query_range("", 0));
    assert_eq!((0, 3), get_initial_query_range("foo", 0));
    assert_eq!((0, 3), get_initial_query_range("foo", 2));
    assert_eq!((0, 3), get_initial_query_range("foo", 3));
    assert_eq!((0, 3), get_initial_query_range("foo bar", 0));
    assert_eq!((0, 3), get_initial_query_range("foo bar", 3));
    assert_eq!((4, 7), get_initial_query_range("foo bar", 4));
    assert_eq!((4, 7), get_initial_query_range("foo bar", 6));
    assert_eq!((4, 7), get_initial_query_range("foo bar", 7));
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

pub fn get_completion(line: String, point: usize, completers: Vec<Box<core::Completer>>)
                      -> io::Result<(String, usize)> {
    let term = termion::get_tty()?;
    let mut model = model::Model::new(completers);

    let (query_start, query_end) = get_initial_query_range(&line, point);
    let original_query = (&line[query_start..query_end]).to_string();
    model.query_set(&original_query);

    let original_terminal_state = terminal::prepare()?;

    let mut term_canvas = canvas::TermCanvas::new(term, CHOOSER_HEIGHT + 1)?;

    model.start_fetching_completions();

    let result : String;

    let (key_sender, key_receiver) = mpsc::channel::<termion::event::Key>();
    let (req_sender, req_receiver) = mpsc::channel::<()>();
    let key_reader_thread = thread::spawn(move || key_reader_thread_routine(req_receiver, key_sender));
    let mut req_sender = Some(req_sender);

    req_sender.as_ref().unwrap().send(()).unwrap();
    loop {
        print_state(&mut term_canvas, &model)?;

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
                        break;
                    }
                },
                Ctrl('c')  => {
                    result = original_query.clone();
                    break;
                },
                Char('\t') => model.next_tab(),
                Char(c)    => model.query_append(c),
                Backspace  => model.query_backspace(),

                _ => {},
            };
            req_sender.as_ref().unwrap().send(()).unwrap();
        }
    }

    req_sender.take();
    key_reader_thread.join().unwrap();

    clear()?;
    terminal::restore(original_terminal_state)?;

    let result_line = format!("{}{}{}", &line[..query_start], &result, &line[query_end..]);
    return Result::Ok((result_line, query_start + result.len()));
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
