use std::env;
use std::io::{self, Write};
use std::process::{self, Command};

mod argparser;
use argparser::{ArgumentParser, Flag};

fn main() {
    let mut ap = ArgumentParser::new(
        "lax",
        "Argument substitution utility",
        "lax [FLAGS] BINARY [ARGS...]",
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

    let args: Vec<String> = env::args().collect();
    let args = ap.process_arguments(&args);

    if args.is_empty() {
        eprintln!("No arguments");
        process::exit(1);
    }

    let mut config = lax::Config {
        match_with_dirs: false,
    };

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
        print!("{}", args.join(" "));
    } else {
        // Go ahead and run the binary with the transformed arguments
        let mut com = Command::new(&args[0]);
        com.args(&args[1..]);
        com.status().expect("Failed!");
    }
}
