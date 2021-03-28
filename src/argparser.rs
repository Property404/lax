// A simple argument parser, because all the existing ones are too complex, yet not flexible enough
// to deal with the use case of ending parsing after finding an argument not beginning with "-"

use std::process;

pub struct Flag {
    // What the client refers to this flag as
    name: &'static str,

    // Description for help text
    description: Option<&'static str>,

    // --long style flag
    long: Option<&'static str>,

    // -sh -or -t style flag
    short: Option<char>,

    // Has this flag been matched yet?
    has: bool,
}

pub struct ArgumentParser {
    // App meta data to display in help/usage text
    name: &'static str,
    description: &'static str,
    usage: &'static str,

    // Could be a hashmap, but we're dealing with a very small number of flags, so this is more
    // efficient
    flags: Vec<Flag>,
}

impl ArgumentParser {
    pub fn new(name: &'static str, description: &'static str, usage: &'static str) -> Self {
        ArgumentParser {
            name,
            description,
            usage,
            flags: Vec::new(),
        }
    }

    pub fn add_flag(mut self, flag: Flag) -> Self {
        self.flags.push(flag);
        self
    }

    pub fn process_argument(&mut self, argument: &str) {
        let is_long = argument.starts_with("--");

        if is_long {
            for flag in &mut (self.flags) {
                if flag.match_against_long(argument) {
                    return;
                }
            }
            eprintln!("Invalid flag \"{}\"", argument);
            process::exit(1);
        }

        for character in (&argument[1..]).chars() {
            let mut matched = false;

            for flag in &mut (self.flags) {
                if flag.match_against_short(character) {
                    matched = true;
                }
            }

            if !matched {
                eprintln!("Invalid flag '{}'", character);
                process::exit(1);
            }
        }
    }

    // Check if a particular flag has been matched
    pub fn has(&self, name: &str) -> bool {
        for flag in &(self.flags) {
            if flag.name == name {
                return flag.has_match();
            }
        }
        panic!("No such flag '{}' exists!", name)
    }

    pub fn help(&self) -> String {
        let mut text: String = format!(
            "{}\n{}\n\nUSAGE:\n    {}\n\nFLAGS:\n",
            self.name, self.description, self.usage
        );

        for flag in &self.flags {
            text = format!("{}    {}\n", text, flag.format());
        }

        text
    }
}

impl Flag {
    pub fn new(name: &'static str) -> Self {
        Flag {
            name,
            long: None,
            short: None,
            description: None,
            has: false,
        }
    }

    pub fn set_short(mut self, short: char) -> Self {
        self.short = Some(short);
        self
    }

    pub fn set_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn set_long(mut self, long: &'static str) -> Self {
        self.long = Some(long);
        self
    }

    pub fn match_against_long(&mut self, pattern: &str) -> bool {
        let long = self.long.unwrap_or("");

        if pattern == long {
            self.has = true;
            return true;
        };

        false
    }

    pub fn match_against_short(&mut self, pattern: char) -> bool {
        if let Some(short) = self.short {
            if pattern == short {
                self.has = true;
                return true;
            }
        };

        false
    }

    pub fn has_match(&self) -> bool {
        self.has
    }

    // Make small summary of flag for use in help menu
    pub fn format(&self) -> String {
        let mut text = String::new();

        if let Some(short) = self.short {
            text = format!("{}-{}", text, short);
            if let Some(_) = self.long {
                text += ", ";
            }
        }

        if let Some(long) = self.long {
            text += long
        }

        format!("{:20}{}", text, self.description.unwrap())
    }
}
