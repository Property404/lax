//! Transform command line arguments by expanding '@' patterns.
#![warn(missing_docs)]
use std::env;
use std::fs;
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
    pub selector_menu: fn(&Vec<String>, bool) -> String,
}

impl Expander {
    /// Expand a entry point/glob pattern pair into all its potential matches.
    fn fetch_matches(
        &self,
        entry_point: &str,
        mut pattern: &str,
        paths: &mut Vec<String>,
    ) -> Result<()> {
        // Not a *super* helpful error message, but I'm unsure when this would come up
        if pattern.is_empty() {
            return Err(anyhow!(
                "No glob pattern specified. \
                               Please see Lax's README for syntax"
            ));
        }

        // Match only with dirs if we end with '/'
        let match_with_dirs = self.config.match_with_dirs;
        let mut match_with_files = self.config.match_with_files;
        if &pattern[pattern.len() - 1..] == "/" {
            pattern = &pattern[0..pattern.len() - 1];
            match_with_files = false;

            if !match_with_dirs {
                return Err(anyhow!(
                    "Matching is configured to only match with \
                                   files, yet glob pattern ends with '/', \
                                   implying a search for directories"
                ));
            }
        }

        let pattern = "./**/".to_string() + pattern;
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
                                               \"@[ENTRY_POINT/**/]GLOB_PATTERN[^SELECTOR]\".\n\tMake sure \
                                               the bit before the first \"/**/\" is a valid \
                                               directory", entry_point));
        }

        // Go to the entry point
        let cwd = env::current_dir()?;
        env::set_current_dir(entry_point)?;

        let walker = WalkDir::new(".").into_iter();
        for e in walker.filter_entry(matcher).filter_map(|e| e.ok()) {
            if let Some(path_name) = e.path().to_str() {
                if glob.is_match(path_name) {
                    // String comparison is a lot faster than fetching the metadata, so keep this
                    // in the inner if block
                    let metadata = e.metadata()?;

                    let matched = (match_with_dirs && match_with_files)
                        || (match_with_dirs && metadata.is_dir())
                        || (match_with_files && metadata.is_file());

                    if matched {
                        paths.push(format!(
                            "{}{}{}",
                            entry_point,
                            &path_name[1..],
                            // Let user know this is a directory
                            if metadata.is_dir() { "/" } else { "" }
                        ));
                    }
                }
            }
        }

        // Head back to our original directory
        env::set_current_dir(cwd)?;

        Ok(())
    }

    // Apply selector to a list of paths.
    //
    // Selectors can be:
    // 0-N: Select path number #n
    // 'a': Select all paths
    // 'l': Select last path
    fn parse_selector(
        mut paths: Vec<String>,
        selector: &str,
    ) -> (Vec<String>, Option<Vec<String>>) {
        let selector = selector.trim();

        // Expand all
        if selector == "a" {
            return (vec![], Some(paths));
        }

        // Expand all
        if selector == "l" {
            return (vec![], Some(vec![paths.remove(paths.len() - 1)]));
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

    // Parse an @ pattern into its subcomponents
    //
    // '@' patterns are in the form:
    // @[ENTRY_POINT/**/]GLOB_PATTERN[^SELECTOR]
    //
    // Where [ENTRY_POINT/**/]GLOB_PATTERN expands into multiple paths, and a selector(possibly SELECTOR) is
    // used to narrow them down
    fn parse_pattern<'a>(&self, pattern: &'a str) -> Result<(&'a str, &'a str, Option<&'a str>)> {
        // Git rid of '@' symbol
        let pattern = &pattern[1..];
        if pattern.is_empty() {
            return Err(anyhow!("Nothing specified after '@' symbol"));
        }

        // Extract selector if it exists
        let mut split: Vec<&str> = pattern.split('^').collect();
        if split.len() > 2 {
            return Err(anyhow!(
                "More than one selector not allowed. \
                Hint: '^' indicates the start of a selector"
            ));
        }
        if split.is_empty() {
            // I -think- this is unreachable. `split` will still always be at leass one item, even
            // if splitting an empty string
            return Err(anyhow!(
                "Unable to extract glob pattern. \
                               This shouldn't happen. Did you do something \
                               weird?\n\nIn any case, you should report this \
                               as a bug."
            ));
        }
        let pattern = split.remove(0);
        let selector = if split.is_empty() {
            None
        } else {
            Some(split.remove(0))
        };

        // Extract entry_point and glob pattern
        let delimiter = "/**/";
        let delimiter_start = pattern.find(delimiter);

        let entry_point;
        let glob_pattern;

        if let Some(delimiter_start) = delimiter_start {
            let delimiter_end = delimiter_start + delimiter.len();

            // Root is an expected default in this case, even if it's not very useful
            entry_point = if delimiter_start == 0 {
                "/"
            } else {
                &pattern[0..delimiter_start]
            };

            // If no glob pattern is given, we should match all directories, since we end with
            // '/**/'
            glob_pattern = if delimiter_end == pattern.len() {
                "*/"
            } else {
                &pattern[delimiter_end..]
            };
        } else {
            // Otherwise, just search in the current directory, which is appropriate for 99% of
            // cases
            entry_point = ".";
            glob_pattern = pattern;
        }

        Ok((entry_point, glob_pattern, selector))
    }

    // Expand an '@' pattern into all its matches, which are narrowed down by either the '@'
    // pattern's selector, or a selector given from a CLI/TUI menu.
    fn expand_pattern(&self, pattern: &str) -> Result<Vec<String>> {
        let (entry_point, glob_pattern, selector) = self.parse_pattern(pattern)?;

        // Get list of all matches
        let mut paths = Vec::new();
        self.fetch_matches(entry_point, glob_pattern, &mut paths)?;

        if paths.is_empty() {
            return Err(anyhow!("Could not match pattern: \"{}\"", pattern));
        }

        // One match - no need to apply the selector
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

            Err(anyhow!("Invalid selector: \"^{}\"", selector))
        } else {
            // No selector - given. Break into CLI or TUI menu
            let mut first_call = true;
            loop {
                let option = (self.selector_menu)(&paths, first_call);
                first_call = false;

                let (all_paths, selected_paths) = Self::parse_selector(paths, &option);

                if let Some(selected_paths) = selected_paths {
                    return Ok(selected_paths);
                }

                paths = all_paths;
            }
        }
    }

    /// Apply post-selector transformations
    ///
    /// # Returns
    /// The transformed and expanded pattern
    fn apply_post_transforms(&self, mut expanded_pattern: Vec<String>) -> Result<Vec<String>> {
        // Transform files to directories
        if self.config.transform_files_to_dirs {
            let res: Result<Vec<String>> = expanded_pattern
                .into_iter()
                .map(|path| {
                    if fs::metadata(&path)?.is_dir() {
                        Ok(path)
                    } else {
                        if let Some(parent) = Path::new(&path).parent() {
                            return Ok(parent.display().to_string());
                        };
                        Err(anyhow!("Could not get parent of file: \"{}\"", path))
                    }
                })
                .collect();
            expanded_pattern = res?;
        };

        Ok(expanded_pattern)
    }

    /// Transform a list of arguments containing 0 or more '@' patterns.
    ///
    /// # Returns
    /// The transformed argument list.
    pub fn expand_arguments(&self, args: &[String]) -> Result<Vec<String>> {
        let mut transformed_args: Vec<String> = Vec::new();
        for arg in args {
            if arg.starts_with('@') {
                let expanded_pattern = self.expand_pattern(&arg)?;
                transformed_args.append(&mut self.apply_post_transforms(expanded_pattern)?);
            } else {
                let new_arg;
                // Allow '@' to be escaped
                if arg.starts_with("\\@") {
                    new_arg = (&arg[1..]).to_string();
                } else {
                    new_arg = arg.to_string();
                }
                transformed_args.push(new_arg);
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
    /// Transform files into their parent directories after selector is applied
    pub transform_files_to_dirs: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}
impl Config {
    /// Construct a default configuration.
    pub fn new() -> Self {
        Config {
            match_with_dirs: true,
            match_with_files: true,
            transform_files_to_dirs: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Expander {
        Expander {
            config: Config::new(),
            selector_menu: |_, _| panic!("Oh god a choice!"),
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
    fn pattern_parsing() {
        let exp = setup();

        let res = exp.parse_pattern("@fish").unwrap();
        assert_eq!(res, (".", "fish", None));

        let res = exp.parse_pattern("@fish^tail").unwrap();
        assert_eq!(res, (".", "fish", Some("tail")));

        let res = exp.parse_pattern("@head/**/fish^tail").unwrap();
        assert_eq!(res, ("head", "fish", Some("tail")));

        let res = exp.parse_pattern("@/**/fish").unwrap();
        assert_eq!(res, ("/", "fish", None));

        let res = exp.parse_pattern("@//**/fish").unwrap();
        assert_eq!(res, ("/", "fish", None));

        let res = exp.parse_pattern("@./**/fish").unwrap();
        assert_eq!(res, (".", "fish", None));

        let res = exp.parse_pattern("@head/**/fish/**/tail").unwrap();
        assert_eq!(res, ("head", "fish/**/tail", None));

        let res = exp.parse_pattern("@head/**/").unwrap();
        assert_eq!(res, ("head", "*/", None));
    }

    // '/' implies matching only directories
    #[test]
    fn imply_directory_matching() {
        let exp = setup();
        let arguments = vec!["@fo*/^a".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded.get(0).unwrap(), "./tests/foobar/");
    }

    #[test]
    fn transform_file_to_parent() {
        let mut exp = setup();
        exp.config.transform_files_to_dirs = true;
        let arguments = vec!["@src/*.rs^1".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded.get(0).unwrap(), "./src");
    }

    #[test]
    fn expand_with_all_selector() {
        let exp = setup();

        let arguments = vec!["@*.rs^a".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert!(expanded.len() > 2);
    }

    #[test]
    fn expand_with_last_selector() {
        let exp = setup();

        let arguments = vec!["@*.rs^l".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 1);
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
