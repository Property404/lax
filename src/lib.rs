//! Transform command line arguments by expanding '@' patterns.
#![warn(missing_docs)]
use std::path::Path;

use globset::GlobBuilder;
use walkdir::{DirEntry, WalkDir};

use anyhow::{anyhow, Result};

/// Struct used to expand '@' patterns.
pub struct Expander {
    /// Configuration object.
    pub config: Config,
    /// A callback function that provides the user with a TUI/CLI menu when a glob pattern matches
    /// more than one result, and no selector is given in the relevant '@' pattern.
    ///
    /// This should return a selector string.
    ///
    /// The first parameter is a list of potential matches.
    /// The second parameter will be true if this is the first time this callback is called for a
    /// particular '@' pattern, and false otherwise. This can be used to provide the user with the
    /// list of matches on first call, but not on the following calls (eg the user enters an
    /// invalid selector)
    pub selector: fn(&Vec<String>, bool) -> String,
}

impl Expander {
    /// Expand a entry point/glob pattern pair into all its potential matches.
    fn fetch_matches(
        &self,
        entry_point: &str,
        pattern: &str,
        paths: &mut Vec<String>,
    ) -> Result<()> {
        let pattern = entry_point.to_string() + "/**/" + pattern;
        let glob = GlobBuilder::new(pattern.as_str())
            .literal_separator(true)
            .build()?
            .compile_matcher();

        // Filter out hidden directories like ".git"
        let matcher = |entry: &DirEntry| {
            let file_name = entry.file_name().to_str();
            let is_hidden = file_name
                .map(|s| s.starts_with('.') && s != "." && s != "..")
                .unwrap_or(false);
            !is_hidden
        };

        if !Path::new(entry_point).exists() {
            return Err(anyhow!("Entry point '{}' doesn't exist.\n\t\
                                               Reminder: the \
                                               @pattern syntax is \
                                               @[ENTRY_POINT/**/]GLOB_PATTERN[^SELECTOR].\n\tMake sure \
                                               the bit before the first '/**/' is a valid \
                                               directory", entry_point));
        }

        let walker = WalkDir::new(entry_point).into_iter();
        for e in walker.filter_entry(matcher).filter_map(|e| e.ok()) {
            let path = e.path();
            if let Some(path_name) = path.to_str() {
                if glob.is_match(path_name) {
                    // String comparison is a lot faster than fetching the metadata, so keep this
                    // in the inner if block
                    let mut matched = self.config.match_with_dirs && self.config.match_with_files;
                    // Actually, we only need to fetch metadata if we're specifically looking
                    // for a file xor directory
                    if !matched {
                        let metadata = e.metadata().unwrap();
                        matched = (self.config.match_with_dirs && metadata.is_dir())
                            || (self.config.match_with_files && metadata.is_file());
                    }

                    if matched {
                        paths.push(path.display().to_string());
                    }
                }
            }
        }

        Ok(())
    }

    // Apply selector to a list of paths.
    //
    // Selectors can be:
    // 0-N: Select path number #n
    // 'a': Select all paths
    fn parse_selector(
        mut paths: Vec<String>,
        selector: &str,
    ) -> (Vec<String>, Option<Vec<String>>) {
        let selector = selector.trim();

        // Expand all
        if selector == "a" {
            return (vec![], Some(paths));
        }

        let index: usize = match selector.parse() {
            Ok(num) => num,
            Err(_) => return (paths, None),
        };

        // Selectors are 1-indexed
        if index < 1 || index > paths.len() {
            return (paths, None);
        }

        (vec![], Some(vec![paths.remove(index - 1)]))
    }

    // Expand an '@' pattern into all its matches, which are narrowed down by either the '@'
    // pattern's selector, or a selector given from a CLI/TUI menu.
    //
    // '@' patterns are in the form:
    // @[ENTRY_POINT/**/]GLOB_PATTERN[^SELECTOR]
    //
    // Where [ENTRY_POINT/**/]GLOB_PATTERN expands into multiple paths, and a selector(possibly SELECTOR) is
    // used to narrow them down
    fn expand_pattern(&self, pattern: &str) -> Result<Vec<String>> {
        // Git rid of '@' symbol
        let pattern = &pattern[1..];

        // extract selector if it exists
        let split: Vec<_> = pattern.split('^').collect();
        if split.len() > 2 {
            return Err(anyhow!(
                "More than one selector not allowed. Hint: '^' indicates the start of a selector"
            ));
        }
        let selector = split.get(1);
        let pattern = split.get(0).unwrap();

        // Extract entry_point and glob pattern
        let split: Vec<_> = pattern.split("/**/").collect();
        let entry_point = if split.len() < 2 { None } else { split.get(0) };
        let glob_pattern = &split[if split.len() < 2 { 0 } else { 1 }..].join("/**/");

        // Get list of all matches
        let mut paths = Vec::new();
        self.fetch_matches(
            entry_point.unwrap_or(&"."),
            glob_pattern.as_str(),
            &mut paths,
        )?;

        if paths.is_empty() {
            return Err(anyhow!("Could not match pattern: @{}", pattern.to_string()));
        }
        if paths.len() == 1 {
            return Ok(vec![paths.remove(0)]);
        }

        // Damn, more than one match
        if let Some(selector) = selector {
            let selector = selector.to_string();
            let (_, selected_paths) = Self::parse_selector(paths, &selector);

            if let Some(selected_paths) = selected_paths {
                return Ok(selected_paths);
            }

            Err(anyhow!("Invalid selector: ^{}", selector))
        } else {
            // No selector - given. Break into CLI or TUI menu
            let mut display_menu = true;
            loop {
                let option = (self.selector)(&paths, display_menu);
                display_menu = false;

                let (all_paths, selected_paths) = Self::parse_selector(paths, &option);

                if let Some(selected_paths) = selected_paths {
                    return Ok(selected_paths);
                }

                paths = all_paths;
            }
        }
    }

    /// Transform a list of arguments containing 0 or more '@' patterns.
    ///
    /// # Returns
    /// The transformed argument list.
    pub fn expand_arguments(&self, args: &[String]) -> Result<Vec<String>> {
        let mut transformed_args: Vec<String> = Vec::new();
        for arg in args {
            if arg.starts_with('@') {
                transformed_args.append(&mut self.expand_pattern(&arg)?);
            } else {
                transformed_args.push(arg.to_string());
            }
        }

        Ok(transformed_args)
    }
}

/// Struct used for configuring an instance of Expander.
pub struct Config {
    /// Do '@' patterns match with directories?
    pub match_with_dirs: bool,
    /// Do '@' patterns match with files?
    pub match_with_files: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Expander {
        Expander {
            config: Config {
                match_with_dirs: true,
                match_with_files: true,
            },
            selector: |_, _| panic!("Oh god a choice!"),
        }
    }
    #[test]
    fn basic() {
        let exp = setup();

        let arguments = vec!["@foo".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded, vec!["./tests/foobar/foo"]);
    }

    #[test]
    fn expand_with_all_selector() {
        let exp = setup();

        let arguments = vec!["@*.rs^a".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert!(expanded.len() > 2);
    }

    #[test]
    fn expand_with_single_selector() {
        let exp = setup();

        let arguments = vec!["@*.rs^1".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 1);
    }

    #[test]
    fn globbing() {
        let exp = setup();
        let patterns_with_many_matches = ["@*.rs^a", "@src/*.rs^a", "@src/../**/*.rs^a"];

        for pattern in &patterns_with_many_matches {
            let arguments = vec![pattern.to_string()];
            let expanded = exp.expand_arguments(&arguments).unwrap();
            assert!(expanded.len() > 2);
        }

        let patterns_with_one_matches = ["@src/main.rs^a", "@foobar/foo^a", "@tests/**/foo^a"];

        for pattern in &patterns_with_one_matches {
            let arguments = vec![pattern.to_string()];
            let expanded = exp.expand_arguments(&arguments).unwrap();
            assert_eq!(expanded.len(), 1);
        }
    }

    // Annoying bug that matches @dep* with @bla/bla/deps/bladfjdkfdf
    // This is undesirable, because if I wanted to look in the deps folder for something, I'd do:
    // @deps/* or @deps/**
    #[test]
    fn dont_match_with_parent_directory() {
        let exp = setup();

        let arguments = vec!["@deps*^a".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        // Bug cause it to match a whole bunch.
        // Should only match two now, but w/e
        assert!(expanded.len() < 4);
        assert!(expanded.len() > 0);
    }
}
