extern crate clap;
extern crate completers;

extern crate log;
extern crate simplelog;

extern crate termion;

use std::fs;
use std::io;
use std::io::Write;
use std::path;

use completers::completers::filesystem;
use completers::completers::git;
use completers::config::WORD_BOUNDARIES;
use completers::core;
use completers::ui;

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

/// Returns the collection of completers to be used for the completion.
///
/// This routine makes it possible to return different sets of completers
/// depending on the query.
fn get_completers(original_query: &str) -> Vec<Box<dyn core::Completer>> {
    let query_path = std::path::PathBuf::from(original_query);
    let fs_completer_path;
    if query_path.is_absolute() {
        // If we start from an absolute path in the query, we interpret
        // that as the user trying to search that directory, not to
        // search for the query as a substring in the current directory.
        fs_completer_path = query_path;
    } else {
        fs_completer_path = std::path::PathBuf::from(".");
    }

    vec![
        Box::new(filesystem::FsCompleter::new(
                path::PathBuf::from(fs_completer_path)
        )),
        Box::new(git::GitBranchCompleter::new()),
    ]
}

fn get_completion_result(line: String,
                         point: usize) -> io::Result<(String, usize)> {
    let (query_start, query_end) = get_initial_query_range(&line, point);
    let original_query = (&line[query_start..query_end]).to_string();

    let completers = get_completers(&original_query);
    let completion = ui::get_completion(&original_query, completers)?;

    let result_line = format!("{}{}{}",
                              &line[..query_start],
                              &completion,
                              &line[query_end..]);
    return Result::Ok((result_line, query_start + completion.len()));
}

fn main() {
    let arguments = clap::App::new("completers")
        .version("0.1.0")
        .author("Sławek Rudnicki <slawek.rudnicki@gmail.com>")
        .about("Extensible interactive completion for *nix shells")
        .arg(clap::Arg::with_name("point")
             .short("p")
             .long("point")
             .value_name("X") // TODO
             .help("Current position of input point within CURRENT_LINE")
             .required(true)
             .takes_value(true))
        .arg(clap::Arg::with_name("CURRENT_LINE")
             .help("The current input line")
             .required(true)
             .index(1))
        .arg(clap::Arg::with_name("debug")
             .long("debug")
             .help("print debug information to /tmp/completers.txt"))
        .get_matches();

    let log_level: log::LevelFilter;
    if arguments.is_present("debug") {
        log_level = log::LevelFilter::Debug;
    } else {
        log_level = log::LevelFilter::Warn;
    }
    simplelog::WriteLogger::init(log_level,
                                 simplelog::Config::default(),
                                 fs::File::create("/tmp/completers.log").unwrap()).unwrap();

    let point: usize = arguments.value_of("point").unwrap().parse().unwrap();
    let line = arguments.value_of("CURRENT_LINE").unwrap().to_string();

    match get_completion_result(line, point) {
        Ok((completion, point)) =>
                writeln!(&mut std::io::stderr(),
                        "{} {}", point, completion)
                        .expect("Failed to write result"),
        Err(error) =>
                writeln!(&mut std::io::stderr(),
                        "{}", error)
                        .expect("Failed to write error description"),
    };
}
