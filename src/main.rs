use std::{
    env,
    io::{self, Write},
    os::unix::process::CommandExt,
    process::{self, Command},
};
mod argparser;

BuildArgumentParser! {
    name: "lax",
    description: "Argument substition utility",
    usage: "lax [FLAGS] BINARY [ARGS...]",

    flags: {
        /// Only match directories
        directories:('d', "--directories"),
        /// Only match files
        files: ('f', "--files"),
        /// Print transformed args to stdout, but don't execute
        print_only: ('p', "--print-only"),
        /// Transform matched files to their parent directory
        file_to_parent: ('D', "--file2parent")
    }
}

fn main() {
    let mut ap = ArgumentParser::default();

    let args: Vec<String> = env::args().collect();
    let args = ap.process_arguments(&args);

    if args.is_empty() {
        eprintln!("lax: No arguments");
        eprintln!("For more information try --help");
        process::exit(1);
    }
    if ap.files && ap.directories {
        eprintln!("The `-d` and `-f` flag can not be on at the same time. They are incompatible.");
        process::exit(1);
    }

    let config = lax::Config {
        transform_files_to_dirs: ap.file_to_parent,
        match_with_files: !ap.directories,
        match_with_dirs: !ap.files,
    };

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

    if ap.print_only {
        print!("{}", args.join(" "));
    } else {
        // Go ahead and run the binary with the transformed arguments
        let programs = &args[0];
        let args = &args[1..];

        // Try multiple programs delimited with '|' in case one doesn't exist.
        let mut err_message = None;
        for program in programs.split('|') {
            let err = Command::new(program).args(args).exec();
            err_message = Some(format!("'{}': {}", program, err));
        }

        // exec() should not have returned
        if let Some(err_message) = err_message {
            eprintln!("lax: {}", err_message);
        } else {
            eprintln!("lax: No program run");
        }
        process::exit(1);
    }
}
