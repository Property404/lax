use std::{
    env,
    io::{self, Write},
    os::unix::process::CommandExt,
    process::{self, Command},
};

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
            .set_description("Only match directories")
            .set_long("--directories")
            .set_short('d'),
    )
    .add_flag(
        Flag::new("files")
            .set_description("Only match files")
            .set_long("--files")
            .set_short('f'),
    )
    .add_flag(
        Flag::new("print_only")
            .set_description("Print transformed args to stdout, but don't execute")
            .set_long("--print-only")
            .set_short('p'),
    )
    .add_flag(
        Flag::new("file_to_parent")
            .set_description("Transform matched files to their parent directory")
            .set_long("--file2parent")
            .set_short('D'),
    );

    let args: Vec<String> = env::args().collect();
    let args = ap.process_arguments(&args);

    if args.is_empty() {
        eprintln!("lax: No arguments");
        eprintln!("For more information try --help");
        process::exit(1);
    }

    let mut config = lax::Config::new();

    if ap.has("directories") {
        config.match_with_files = false;
    }
    if ap.has("file_to_parent") {
        config.transform_files_to_dirs = true;
    }
    if ap.has("files") {
        config.match_with_dirs = false;
    }
    if !config.match_with_dirs && !config.match_with_files {
        eprintln!("The `-d` and `-f` flag can not be on at the same time. They are incompatible.");
        process::exit(1);
    }

    // After this, we only do '@' transformations
    let expander = lax::Expander {
        config,
        selector_menu: |paths, first_call| {
            if first_call {
                eprintln!("Found the following:");
                eprintln!("====================");
                for (i, path) in paths.iter().enumerate() {
                    eprintln!("{}. {}", i + 1, path);
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

            // Allow user to quit
            if option.starts_with('q') {
                process::exit(1);
            }

            option
        },
    };

    let args = match expander.expand_arguments(args) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("lax: {}", err);
            process::exit(1)
        }
    };

    if ap.has("print_only") {
        print!("{}", args.join(" "));
    } else {
        // Go ahead and run the binary with the transformed arguments
        let program = &args[0];
        let args = &args[1..];

        // Shouldn't return
        let err = Command::new(program).args(args).exec();
        eprintln!("Failed to execute '{}': {}", program, err);
        process::exit(1);
    }
}
