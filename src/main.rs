extern crate clap;
extern crate completers;
extern crate log;
extern crate simplelog;

use std::fs;
use std::io;
use std::io::Write;
use std::path;

use completers::completers::filesystem;
use completers::completers::git;
use completers::core;
use completers::ui;

fn complete(line: String, point: usize) -> io::Result<(String, usize)> {
    let completers: Vec<Box<core::Completer>> = vec![
        Box::new(filesystem::FsCompleter::new(path::PathBuf::from("."))),
        Box::new(git::GitBranchCompleter::new()),
    ];
    return ui::get_completion(line, point, completers);
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
        .arg(clap::Arg::with_name("debug"))
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

    match complete(line, point) {
        Ok((completion, point)) => println!("{} {}", point, completion),
        Err(error) => writeln!(&mut std::io::stderr(), "{}", error).expect("Failed to write!"),
    };
}
