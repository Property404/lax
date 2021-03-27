extern crate walkdir;
use std::env;
use std::io;
use std::io::Write;
use std::process::Command;
use walkdir::WalkDir;

fn fetch_matches(pattern: &String, paths: &mut Vec<String>) {
    for e in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        if e.metadata().unwrap().is_file() {
            let e = e.path();

            match e.file_name() {
                Some(file_name) => match file_name.to_str() {
                    Some(s) => {
                        let s = String::from(s);
                        if !s.starts_with(".") && format!("@{}", s) == *pattern {
                            paths.push(e.display().to_string());
                        }
                    }
                    None => (),
                },
                None => (),
            }
        }
    }
}

fn locate_target(pattern: &String) -> Option<String> {
    // Get list of all matches
    let mut paths = Vec::new();
    fetch_matches(pattern, &mut paths);

    if paths.len() == 0 {
        println!("No match for '{}'", pattern);
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

fn main() {
    let mut args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("No arguments");
        return;
    }

    for arg in &mut args {
        if arg.starts_with("@") {
            if let Some(target) = locate_target(arg) {
                *arg = target;
            } else {
                eprintln!("Couldn't find {}", &arg[1..]);
                return;
            }
        }
    }

    let mut com = Command::new(&args[1]);
    com.args(&args[2..]);
    com.status().expect("Failed!");
}
