//! A simple argument parser, because all the existing ones are too complex, yet not flexible enough
//! to deal with the use case of ending parsing after finding an argument not beginning with "-".
//!
//! Methods in this module exit upon failure.

use std::process;

/// A command line flag.
pub struct Flag {
    /// What the client refers to this flag as
    name: &'static str,

    /// Description for help text
    description: Option<&'static str>,

    /// --long style flag
    long: Option<&'static str>,

    /// -sh -or -t style flag
    short: Option<char>,

    /// Has this flag been matched yet?
    has: bool,
}

pub struct ArgumentParser {
    /// Name of this program
    name: &'static str,

    /// Description of this program
    description: &'static str,

    /// Usage text of this program
    usage: &'static str,

    /// All possible flags the user can pass
    flags: Vec<Flag>,
}

impl ArgumentParser {
    /// Constructor.
    pub fn new(name: &'static str, description: &'static str, usage: &'static str) -> Self {
        ArgumentParser {
            name,
            description,
            usage,
            flags: Vec::new(),
        }
    }

    /// Add a processable flag
    pub fn add_flag(mut self, flag: Flag) -> Self {
        self.flags.push(flag);
        self
    }

    /// Process a single argument. Determine what flag it's associated with and fail if there's no
    /// associated flag.
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

    /// Returns true if a particular flag has been matched.
    ///
    /// # Panics
    /// Panics if flag doesn't exist.
    pub fn has(&self, name: &str) -> bool {
        for flag in &(self.flags) {
            if flag.name == name {
                return flag.has_match();
            }
        }
        panic!("No such flag '{}' exists!", name)
    }

    /// Return the help text.
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
    /// Constructor.
    pub fn new(name: &'static str) -> Self {
        Flag {
            name,
            long: None,
            short: None,
            description: None,
            has: false,
        }
    }

    /// Set the character used as the short name of this flag. E.g. 'c' for '-c'.
    pub fn set_short(mut self, short: char) -> Self {
        self.short = Some(short);
        self
    }

    /// Set the long name, e.g. "--long". Be sure to include the "--".
    pub fn set_long(mut self, long: &'static str) -> Self {
        self.long = Some(long);
        self
    }

    /// Set the description to be used in the help/usage message.
    pub fn set_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    /// Check if long name matches the given string, including the "--". Later, the client can call
    /// `has_match()` which will return true if(but not only if) it had matched.
    pub fn match_against_long(&mut self, pattern: &str) -> bool {
        let long = self.long.unwrap_or("");

        if pattern == long {
            self.has = true;
            return true;
        };

        false
    }

    /// Check if short name matches the given char. Later, the client can call `has_match()` which
    /// will return true if(but not only if) it had matched.
    pub fn match_against_short(&mut self, pattern: char) -> bool {
        if let Some(short) = self.short {
            if pattern == short {
                self.has = true;
                return true;
            }
        };

        false
    }

    /// Returns true if this flag has been matched before.
    pub fn has_match(&self) -> bool {
        self.has
    }

    /// Make small summary of flag for use in help menu.
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
