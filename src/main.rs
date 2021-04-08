use std::env;
use std::io::{self, Write};
use std::process::{self, Command};

mod argparser;
use argparser::{ArgumentParser, Flag};

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let mut ap = ArgumentParser::new(
        "lax",
        "Argument substitution utility",
        "lax [FLAGS] BINARY [ARGS...]",
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
    )
    .add_flag(
        Flag::new("print_only")
            .set_description("Print transformed args to stdout, but don't execute")
            .set_long("--print-only")
            .set_short('p'),
    );

    if args.len() < 2 {
        eprintln!("No arguments");
        process::exit(1);
    }

    let mut config = lax::Config {
        match_with_dirs: false,
    };

    // Where the binary is located as an index within args
    let mut command_location = 1;

    // Consider the first flags to be flags for lax itself, until a non-flag is found
    for arg in &mut args[1..] {
        // Explicitly stop processing args
        if arg == "--" {
            command_location += 1;
            break;
        }

        if arg.starts_with('-') {
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
    let expander = lax::Expander {
        config,
        selector: |paths, display_menu| {
            if display_menu {
                eprintln!("Found the following files");
                eprintln!("=========================");
                for (i, path) in paths.iter().enumerate() {
                    eprintln!("{}. {}", i, path);
                }
            }

            eprint!("Select> ");
            match io::stdout().flush() {
                Ok(_) => (),
                Err(error) => eprintln!("Error: {}", error),
            };
            let mut option = String::new();
            io::stdin()
                .read_line(&mut option)
                .expect("Failed to read from stdin");
            option
        },
    };
    let args = match expander.expand_arguments(args) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1)
        }
    };

    if ap.has("print_only") {
        print!("{}", &args[command_location..].join(" "));
    } else {
        // Go ahead and run the binary with the transformed arguments
        let mut com = Command::new(&args[command_location]);
        com.args(&args[command_location + 1..]);
        com.status().expect("Failed!");
    }
}
