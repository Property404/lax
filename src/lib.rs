//! Transform command line arguments by expanding '@' patterns.
#![warn(missing_docs)]

use globset::Glob;
use walkdir::{DirEntry, WalkDir};

mod errors;
pub use errors::{LaxError, LaxResult};

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
    /// Expand a glob pattern into all its potential matches.
    fn fetch_matches(&self, pattern: &str, paths: &mut Vec<String>) -> LaxResult {
        let glob = Glob::new(pattern)?.compile_matcher();

        // Filter out hidden directories like ".git"
        let matcher = |entry: &DirEntry| {
            let file_name = entry.file_name().to_str();
            let is_hidden = file_name
                .map(|s| s.starts_with('.') && s != ".")
                .unwrap_or(false);
            !is_hidden
        };

        let walker = WalkDir::new(".").into_iter();
        for e in walker.filter_entry(matcher).filter_map(|e| e.ok()) {
            let path = e.path();
            if let Some(file_name) = path.file_name() {
                if let Some(file_name) = file_name.to_str() {
                    if glob.is_match(file_name) {
                        // String comparison is a lot faster than fetching the metadata, so keep this
                        // in the inner if block
                        if self.config.match_with_dirs || e.metadata().unwrap().is_file() {
                            paths.push(path.display().to_string());
                        }
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
    // @GLOB_PATTERN[^SELECTOR]
    //
    // Where GLOB_PATTERN expands into multiple paths, and a selector(possibly SELECTOR) is used to
    // narrow them down
    fn expand_pattern(&self, pattern: &str) -> LaxResult<Vec<String>> {
        // Find selector in pattern
        let mut selector_point = pattern.len();
        let mut selector = None;
        if let Some(new_selector_point) = pattern.rfind('^') {
            selector_point = new_selector_point;
            selector = Some(&pattern[selector_point + 1..]);
        }

        // Get list of all matches
        let mut paths = Vec::new();
        self.fetch_matches(&pattern[1..selector_point], &mut paths)?;

        if paths.is_empty() {
            return Err(LaxError::EntityNotFound(pattern.to_string()));
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

            Err(LaxError::InvalidSelector(selector))
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
    pub fn expand_arguments(&self, args: &[String]) -> LaxResult<Vec<String>> {
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
    /// Do '@' patterns match with directories, or only files?
    pub match_with_dirs: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Expander {
        Expander {
            config: Config {
                match_with_dirs: false,
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

        let arguments = vec!["@*.rs^0".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 1);
    }
}
