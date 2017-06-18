extern crate clap;
extern crate ctrlc;
extern crate completers;

use std::io;
use std::io::Write;
use std::path;

use completers::completers::filesystem;
use completers::ui;


fn complete(line: String) -> io::Result<(String, i16)> {
    let completer = Box::new(filesystem::FsCompleter::new(path::PathBuf::from(".")));
    return ui::get_completion(line, completer);
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
        .get_matches();

    let point: i16 = arguments.value_of("point").unwrap().parse().unwrap();
    let line = arguments.value_of("CURRENT_LINE").unwrap().to_string();

    match complete(line) {
        Ok((completion, point_move)) => println!("{} {}", point + point_move, completion),
        Err(error) => writeln!(&mut std::io::stderr(), "{}", error).expect("Failed to write!"),
    };
}
