use globset::Glob;
use walkdir::{DirEntry, WalkDir};

mod errors;
pub use errors::{LaxError, LaxResult};

pub struct Expander {
    pub config: Config,
    pub selector: fn(&Vec<String>, bool) -> String,
}

impl Expander {
    fn fetch_matches(&self, pattern: &str, paths: &mut Vec<String>) -> LaxResult {
        let glob = Glob::new(pattern)?.compile_matcher();

        // Filter out hidden directories like ".git"
        let matcher = |entry: &DirEntry| {
            let file_name = entry.file_name().to_str();
            let is_hidden = file_name
                .map(|s| s.starts_with(".") && s != ".")
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

    fn parse_selector(
        mut paths: Vec<String>,
        selector: &String,
    ) -> (Vec<String>, Option<Vec<String>>) {
        // Expand all
        if selector == "a" {
            return (vec![], Some(paths));
        }

        let index = selector.trim();
        let index: usize = match index.parse() {
            Ok(num) => num,
            Err(_) => return (paths, None),
        };

        if index >= paths.len() {
            return (paths, None);
        }

        return (vec![], Some(vec![paths.remove(index)]));
    }

    fn expand_pattern(&self, pattern: &String) -> LaxResult<Vec<String>> {
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

        if paths.len() == 0 {
            return Err(LaxError::EntityNotFound(pattern.clone()));
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

            return Err(LaxError::SelectorParsing(selector));
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

    pub fn expand_arguments(&self, args: Vec<String>) -> LaxResult<Vec<String>> {
        let mut transformed_args: Vec<String> = Vec::new();
        for arg in args {
            if arg.starts_with("@") {
                transformed_args.append(&mut self.expand_pattern(&arg)?);
            } else {
                transformed_args.push(arg);
            }
        }

        Ok(transformed_args)
    }
}

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
        let expanded = exp.expand_arguments(arguments).unwrap();
        assert_eq!(expanded, vec!["./tests/foobar/foo"]);
    }

    #[test]
    fn expand_with_all_selector() {
        let exp = setup();

        let arguments = vec!["@*.rs^a".to_string()];
        let expanded = exp.expand_arguments(arguments).unwrap();
        assert!(expanded.len() > 2);
    }

    #[test]
    fn expand_with_single_selector() {
        let exp = setup();

        let arguments = vec!["@*.rs^0".to_string()];
        let expanded = exp.expand_arguments(arguments).unwrap();
        assert_eq!(expanded.len(), 1);
    }
}
