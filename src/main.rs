extern crate clap;
extern crate ctrlc;
extern crate completers;

use std::io;
use std::io::Write;
use std::process;

use completers::ui;

fn complete(line: &str) -> io::Result<String> {
    ctrlc::set_handler(|| {
        ui::clear().unwrap();
        println!("");
        process::exit(0);
    }).unwrap();
    
    let propositions: Vec<String> = (1..100).map(|n| format!("{}", n)).collect();
    let proposition_refs: Vec<&str> = propositions.iter().map(|s| s.as_str()).collect();
    return ui::get_completion(line, &proposition_refs);
}

fn main() {
    let arguments = clap::App::new("completers")
        .version("0.1.0")
        .author("Sławek Rudnicki <slawek.rudnicki@gmail.com>")
        .about("Extensible interactive completion for *nix shells")
        .arg(clap::Arg::with_name("point")
             .short("p")
             .long("point")
             .value_name("X:Y") // TODO
             .help("Current position of input point")
             .required(true)
             .takes_value(true))
        .arg(clap::Arg::with_name("CURRENT_LINE")
             .help("The current input line")
             .required(true)
             .index(1))
        .get_matches();

    let point = arguments.value_of("point").unwrap();
    let line = arguments.value_of("CURRENT_LINE").unwrap();

    match complete(&line) {
        Ok(completion) => println!("{}", completion),
        Err(error) => writeln!(&mut std::io::stderr(), "{}", error).expect("Failed to write!"),
    };
}
