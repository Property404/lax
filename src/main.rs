use std::env;
use std::io;
use std::io::Write;
use std::process::{self, Command};
use walkdir::{DirEntry, WalkDir};

mod argparser;
use argparser::{ArgumentParser, Flag};

fn fetch_matches(pattern: &String, paths: &mut Vec<String>, config: &Config) {
    // Remove the "@" symbol;
    let pattern = &pattern[1..];

    // Filter out hidden directories like ".git"
    let matcher = |entry: &DirEntry| {
        let file_name = entry.file_name().to_str();
        let is_hidden = file_name
            .map(|s| s.starts_with(".") && s != ".")
            .unwrap_or(false);
        !is_hidden
    };

    let walker = WalkDir::new(".").into_iter();
    for e in walker.filter_entry(matcher).filter_map(|e| e.ok()) {
        let path = e.path();
        if let Some(file_name) = path.file_name() {
            if let Some(file_name) = file_name.to_str() {
                if file_name == pattern {
                    // String comparison is a lot faster than fetching the metadata, so keep this
                    // in the inner if block
                    if config.match_with_dirs || e.metadata().unwrap().is_file() {
                        paths.push(path.display().to_string());
                    }
                }
            }
        }
    }
}

fn locate_target(pattern: &String, config: &Config) -> Option<String> {
    // Get list of all matches
    let mut paths = Vec::new();
    fetch_matches(pattern, &mut paths, config);

    if paths.len() == 0 {
        return None;
    }

    let mut target_id = 0;
    if paths.len() > 1 {
        println!("Found the following files");
        println!("=========================");
        let mut i = 0;
        for path in &paths {
            println!("{}. {}", i, path);
            i += 1;
        }

        loop {
            print!("Select> ");
            match io::stdout().flush() {
                Ok(_) => (),
                Err(error) => println!("Error: {}", error),
            };
            let mut option = String::new();
            io::stdin()
                .read_line(&mut option)
                .expect("Failed to read from stdin");
            let option = option.trim();

            let option: usize = match option.parse() {
                Ok(num) => num,
                Err(_) => continue,
            };

            if option >= paths.len() {
                continue;
            }

            target_id = option;
            break;
        }
    };

    let target = paths.remove(target_id);

    Some(target)
}

struct Config {
    // Do '@' patterns match with directories, or only files?
    match_with_dirs: bool,
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let mut ap = ArgumentParser::new(
        "lex",
        "Argument substitution utility",
        "lex [FLAGS] BINARY [ARGS...]",
    )
    .add_flag(
        Flag::new("help")
            .set_description("Display this message")
            .set_long("--help")
            .set_short('h'),
    )
    .add_flag(
        Flag::new("directories")
            .set_description("Match directories")
            .set_long("--directories")
            .set_short('d'),
    );

    if args.len() < 2 {
        eprintln!("No arguments");
        process::exit(1);
    }

    let mut config = Config {
        match_with_dirs: false,
    };

    // Where the binary is located as an index within args
    let mut command_location = 1;

    // Consider the first flags to be flags for lex itself, until a non-flag is found
    for arg in &mut args[1..] {
        if arg.starts_with("-") {
            ap.process_argument(arg.as_str());
            command_location += 1;
            continue;
        };
        break;
    }
    if ap.has("help") {
        println!("{}", ap.help());
        return;
    }
    if ap.has("directories") {
        config.match_with_dirs = true;
    }

    // After this, we only do '@' transformations
    for arg in &mut args[command_location..] {
        if arg.starts_with("@") {
            if let Some(target) = locate_target(arg, &config) {
                *arg = target;
            } else {
                eprintln!("Couldn't find file '{}'", &arg[1..]);
                process::exit(1);
            }
        }
    }

    // Go ahead and run the binary with the transformed arguments
    let mut com = Command::new(&args[command_location]);
    com.args(&args[command_location + 1..]);
    com.status().expect("Failed!");
}
