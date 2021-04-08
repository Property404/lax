use globset::Glob;
use walkdir::{DirEntry, WalkDir};

mod errors;
pub use errors::{LaxError, LaxResult};

pub struct Expander {
    pub config: Config,
    pub selector: fn(Vec<String>) -> Vec<String>,
}

impl Expander {
    fn fetch_matches(&self, pattern: &String, paths: &mut Vec<String>) -> LaxResult {
        // Remove the "@" symbol;
        let pattern = &pattern[1..];
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

    fn expand_pattern(&self, pattern: &String) -> LaxResult<Vec<String>> {
        // Get list of all matches
        let mut paths = Vec::new();
        self.fetch_matches(pattern, &mut paths)?;

        if paths.len() == 0 {
            return Err(LaxError::EntityNotFound(pattern.clone()));
        }

        if paths.len() == 1 {
            return Ok(vec![paths.remove(0)]);
        }

        Ok((self.selector)(paths))
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
