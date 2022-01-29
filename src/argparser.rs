//! A simple argument parser, because all the existing ones are too complex, yet not flexible enough
//! to deal with the use case of ending parsing after finding an argument not beginning with "-".
//!
//! Methods in this module exit upon failure.

#[macro_export]
macro_rules! BuildArgumentParser {
    (
        name: $name:literal,
        description: $description:literal,
        usage: $usage:literal,

        flags: {
            $(
                #[doc = $flag_description:expr]
                $flag: ident: ($short: literal, $long:literal)
            ),*
        }
    ) => {
        BuildArgumentParser!{@
            $name,
            $description,
            $usage,

            $(
                #[doc = $flag_description]
                $flag:  ($short, $long),
            )*
            /// Print help information
            help: ('h', "--help"),
            /// Print version info and exit
            version: ('V', "--version")
        }
    };
    (@
        $name:literal,
        $description:literal,
        $usage:literal,

        $(
            #[doc = $flag_description:expr]
            $flag: ident: ($short: literal, $long:literal)
        ),*
    ) => {
        #[derive(Default)]
        pub struct ArgumentParser {
            $(
                $flag: bool
            ),*
        }
        impl ArgumentParser {
            /// Process a single argument. Determine what flag it's associated with and fail if there's no
            /// associated flag.
            fn process_argument(&mut self, argument: &str) {
                let is_long = argument.starts_with("--");

                if is_long {
                    match argument {
                        $(
                           $long => { self.$flag = true }
                        ),*
                        _ => {
                            eprintln!("Invalid flag '{}'", argument);
                            std::process::exit(1);
                        }
                    };
                    return;
                }

                for character in (&argument[1..]).chars() {
                    match character {
                        $(
                           $short => { self.$flag = true }
                        ),*
                        _ => {
                            eprintln!("Invalid flag '{}'", character);
                            std::process::exit(1);
                        }
                    };
                }
            }

            /// Process a list of arguments up until the first non-flag is found,
            /// then return the flagless part of the vector
            pub fn process_arguments<'a>(&mut self, arguments: &'a [String]) -> &'a [String] {
                // Very first argument is just the name, so skip it
                let mut position: usize = 1;

                for arg in &arguments[position..] {
                    // Explicitly stop processing args
                    if arg == "--" {
                        position += 1;
                        break;
                    }

                    if arg.starts_with('-') {
                        self.process_argument(arg.as_str());
                        position += 1;
                        continue;
                    };
                    break;
                }

                if self.help {
                    println!(
                        "{}\n{}\n\nUSAGE:\n    {}\n\nFLAGS:\n",
                        $name, $description, $usage
                    );

                    $(
                        println!("    -{}, {:15}{}", $short, $long, $flag_description);
                    )*

                    std::process::exit(0);
                };

                if self.version {
                    println!("{} {}", $name, env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                };

                &arguments[position..]
            }
        }
    }
}

#[cfg(test)]
mod test {
    BuildArgumentParser! {
        name: "mock",
        description: "Mock program",
        usage: "mock [FLAGS] BINARY [ARGS...]",

        flags: {
            /// Turn flag 1 on
            flag1:('1', "--flag1"),
            /// Turn flag 2 on
            flag2:('2', "--flag2")
        }
    }

    #[test]
    fn argument_parsing() {
        let mut ap = ArgumentParser::default();
        let args = ["mock", "-1", "--flag2"].map(String::from);
        ap.process_arguments(&args);
        assert!(ap.flag1);
        assert!(ap.flag2);

        let mut ap = ArgumentParser::default();
        let args = ["mock", "-1"].map(String::from);
        ap.process_arguments(&args);
        assert!(ap.flag1);
        assert!(!ap.flag2);

        let mut ap = ArgumentParser::default();
        let args = ["mock", "-12"].map(String::from);
        ap.process_arguments(&args);
        assert!(ap.flag1);
        assert!(ap.flag2);

        let mut ap = ArgumentParser::default();
        let args = ["mock", "-21"].map(String::from);
        ap.process_arguments(&args);
        assert!(ap.flag1);
        assert!(ap.flag2);

        let mut ap = ArgumentParser::default();
        let args = ["mock", "-2"].map(String::from);
        ap.process_arguments(&args);
        assert!(!ap.flag1);
        assert!(ap.flag2);
    }
}
