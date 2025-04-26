//! Transform command line arguments by expanding '@' patterns.
#![warn(missing_docs)]
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
use globset::GlobBuilder;
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

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
    pub selector_menu: fn(paths: &[String], first_call: bool) -> String,
}

#[derive(PartialEq, Debug)]
enum Selector {
    All,
    FromFront(usize),
    FromBack(usize),
    Regex(String),
}
#[derive(PartialEq, Debug)]
struct SelectorGroup {
    selectors: Vec<Selector>,
}

impl SelectorGroup {
    // Select all paths that match the selector group.
    fn select(&self, paths: &[String]) -> Result<Vec<String>> {
        let mut selected_paths = Vec::<String>::new();
        for selector in &self.selectors {
            if paths.is_empty() {
                return Err(anyhow!("No paths to select!"));
            }
            match selector {
                Selector::All => {
                    selected_paths.extend(paths.to_owned());
                }
                Selector::FromFront(offset) => {
                    if *offset >= paths.len() {
                        return Err(anyhow!("Selector index out of range: {}", offset + 1));
                    }
                    selected_paths.push(paths[*offset].clone());
                }
                Selector::FromBack(offset) => {
                    if *offset >= paths.len() {
                        return Err(anyhow!("Selector index out of range: -{}", offset + 1));
                    }
                    selected_paths.push(paths[paths.len() - 1 - offset].clone());
                }
                Selector::Regex(regex) => {
                    let regex = Regex::new(regex)?;
                    selected_paths.extend(paths.iter().filter(|v| regex.is_match(v)).cloned());
                }
            }
        }

        Ok(selected_paths)
    }

    // Return highest index we will select, with no knowledge of how long the list of paths will
    // be. None implies infinity
    fn highest_index(&self) -> Option<usize> {
        let mut highest_index = 0;
        for selector in &self.selectors {
            match selector {
                Selector::FromFront(offset) => {
                    highest_index = std::cmp::max(*offset, highest_index);
                }
                Selector::FromBack(_) | Selector::All | Selector::Regex(_) => {
                    return None;
                }
            }
        }
        Some(highest_index)
    }
}

impl Expander {
    /// Expand a entry point/glob pattern pair into all its potential matches.
    fn fetch_matches(
        &self,
        from_repository_root: bool,
        entry_point: &str,
        mut pattern: &str,
        paths: &mut Vec<String>,
        selector_group: &Option<SelectorGroup>,
    ) -> Result<()> {
        if pattern.is_empty() {
            // This way we can `cd @%` to cd to the repository root
            if from_repository_root {
                paths.push(get_repository_root()?.to_string_lossy().into_owned());
                return Ok(());
            }

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

        // Filter out hidden directories like ".git"/".svn"
        let matcher = match self.config.search_hidden {
            true => |_: &DirEntry| true,
            false => |entry: &DirEntry| {
                let file_name = entry.file_name().to_str();
                let is_hidden = file_name
                    .map(|s| s.starts_with('.') && s != "." && s != "..")
                    .unwrap_or(false);
                !is_hidden
            },
        };

        let entry_point = shellexpand::tilde(entry_point);
        let entry_point = entry_point.as_ref();

        // Possibly need to find the git/svn root
        let entry_point = if from_repository_root {
            let root = get_repository_root()?;
            if entry_point != "." && entry_point != "/" {
                root.join(entry_point)
            } else {
                root
            }
        } else {
            PathBuf::from(entry_point)
        };

        if !entry_point.exists() {
            return Err(anyhow!("Entry point {:?} doesn't exist.\n\t\
                                               Reminder: the \
                                               @pattern syntax is \
                                               \"@[%][ENTRY_POINT/**/]GLOB_PATTERN[^SELECTOR]\".\n\tMake sure \
                                               the bit before the first \"/**/\" is a valid \
                                               directory", entry_point));
        }

        // Go to the entry point
        let cwd = env::current_dir()?;
        env::set_current_dir(&entry_point)?;

        // We have an opportunity to quit early in some cases when selectors are provided.
        let quit_after_index = match selector_group {
            Some(selector_group) => selector_group.highest_index(),
            None => None,
        };
        let mut current_index = 0;

        let walker = WalkDir::new(".").into_iter();
        for e in walker.filter_entry(matcher).filter_map(|e| e.ok()) {
            if let Some(path_name) = e.path().to_str() {
                if glob.is_match(path_name) {
                    // String comparison is a lot faster than fetching the metadata, so keep this
                    // in the inner if block
                    let metadata = e.metadata()?;

                    let matched = (match_with_dirs && (match_with_files || metadata.is_dir()))
                        || (match_with_files && metadata.is_file());

                    if matched {
                        let path_name = match path_name.strip_prefix("./") {
                            Some(path_name) => path_name,
                            None => path_name,
                        };
                        let mut result = entry_point.join(path_name).to_string_lossy().to_string();
                        if metadata.is_dir() {
                            result.push('/')
                        }
                        paths.push(result);

                        if let Some(quit_after_index) = quit_after_index {
                            if quit_after_index == current_index {
                                return Ok(());
                            }

                            current_index += 1;
                        }
                    }
                }
            }
        }

        // Head back to our original directory
        env::set_current_dir(cwd)?;

        Ok(())
    }

    // Build a selector group from string.
    //
    // Selectors can be:
    // 1 to N: Select path number #n
    // -N to -1: Select path number #n in reverse order
    // 'a': Select all paths
    // 'l': Select last path
    //
    // Multiple selectors are delimited by commas.
    fn parse_selectors(raw_selectors: &str) -> Result<SelectorGroup> {
        let mut selectors = vec![];

        for selector in raw_selectors.trim().split(',') {
            if selector == "a" {
                selectors.push(Selector::All);
                continue;
            }

            if let Some(selector) = selector.strip_prefix('/') {
                selectors.push(Selector::Regex(selector.into()));
                continue;
            }

            // This was added before you could specify negative selectors. Consider deprecation.
            if selector == "l" {
                selectors.push(Selector::FromBack(0));
                continue;
            }

            let index: isize = selector
                .parse()
                .map_err(|_| anyhow!("Invalid selector: '{selector}'"))?;

            // Selectors are 1-indexed
            if index == 0 {
                return Err(anyhow!("Selectors are 1-indexed and cannot be zero"));
            }

            if index < 0 {
                selectors.push(Selector::FromBack(index.unsigned_abs() - 1));
            } else {
                selectors.push(Selector::FromFront(index.unsigned_abs() - 1));
            }
        }
        Ok(SelectorGroup { selectors })
    }

    // Parse an @ pattern into its subcomponents
    //
    // '@' patterns are in the form:
    // @[%][ENTRY_POINT/**/]GLOB_PATTERN[^SELECTOR_GROUP]
    //
    // Where [%][ENTRY_POINT/**/]GLOB_PATTERN expands into multiple paths, and a selector
    // group(possibly SELECTOR_GROUP) is used to narrow them down
    fn parse_pattern(pattern: &str) -> Result<(bool, &str, &str, Option<&str>)> {
        // Git rid of '@' symbol
        let pattern = &pattern[1..];

        if pattern.is_empty() {
            bail!("Empty pattern - nothing specified after '@' symbol");
        }

        // The "from repository root" modifier. This enables us to start the search from the git/svn root.
        let (pattern, repository_root) = if let Some(pattern) = pattern.strip_prefix('%') {
            (pattern, true)
        // Faux "escape modifier" modifier, so we can escape what would otherwise be considered a
        // modifier
        } else if let Some(pattern) = pattern.strip_prefix('\\') {
            (pattern, false)
        } else {
            (pattern, false)
        };

        let pattern = &mut pattern.split('^');

        let (pattern, selectors) = (
            pattern
                .next()
                .ok_or_else(|| anyhow!("Empty patterns are not allowed"))?,
            pattern.next(),
        );

        // Extract entry_point and glob pattern
        let mut pattern = pattern.splitn(2, "/**/");

        let (entry_point, glob_pattern) = match (pattern.next(), pattern.next()) {
            (Some(glob_pattern), None) => (".", glob_pattern),
            (Some(entry_point), Some(glob_pattern)) => (
                // Root is an expected default in this case, even if it's not very useful
                if entry_point.is_empty() {
                    "/"
                } else {
                    entry_point
                },
                // If no glob pattern is given, we should match all directories, since we end with
                // '/**/'
                if glob_pattern.is_empty() {
                    "*/"
                } else {
                    glob_pattern
                },
            ),
            // .splitn(2,_) will produce at least one value, even on an empty string
            (None, _) => unreachable!(),
        };

        Ok((repository_root, entry_point, glob_pattern, selectors))
    }

    // Expand an '@' pattern into all its matches, which are narrowed down by either the '@'
    // pattern's selectors, or selectors given from a CLI/TUI menu.
    fn expand_pattern(&self, pattern: &str) -> Result<Vec<String>> {
        let (repository_root, entry_point, glob_pattern, selector_group) =
            Self::parse_pattern(pattern)?;
        let selector_group = selector_group.map(Self::parse_selectors).transpose()?;

        // Get list of all matches
        let mut paths = Vec::new();
        self.fetch_matches(
            repository_root,
            entry_point,
            glob_pattern,
            &mut paths,
            &selector_group,
        )?;

        if paths.is_empty() {
            return Err(anyhow!("Could not match pattern: \"{}\"", glob_pattern));
        }

        if let Some(selector_group) = selector_group {
            selector_group.select(&paths)
        } else {
            // One match - no need to bother the user.
            if paths.len() == 1 {
                return Ok(vec![paths.remove(0)]);
            }

            // No selector - given. Break into CLI or TUI menu
            let mut first_call = true;
            loop {
                let option = (self.selector_menu)(&paths, first_call);
                first_call = false;

                let selected_paths = Self::parse_selectors(&option)?.select(&paths);

                if let Ok(selected_paths) = selected_paths {
                    return Ok(selected_paths);
                }
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
                    } else if let Some(parent) = Path::new(&path).parent() {
                        Ok(parent.display().to_string())
                    } else {
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
                let expanded_pattern = self.expand_pattern(arg)?;
                transformed_args.append(&mut self.apply_post_transforms(expanded_pattern)?);
            } else {
                // Allow '@' to be escaped
                let new_arg = if arg.starts_with("\\@") {
                    arg[1..].to_string()
                } else {
                    arg.to_string()
                };
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
    /// Transform files into their parent directories after selectors are applied
    pub transform_files_to_dirs: bool,
    /// Should we search hidden files/directories?
    pub search_hidden: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            match_with_dirs: true,
            match_with_files: true,
            transform_files_to_dirs: false,
            search_hidden: false,
        }
    }
}

fn get_repository_root() -> Result<PathBuf> {
    let mut cwd = env::current_dir()?;
    while !cwd.join(".git").exists() && !cwd.join(".svn").exists() {
        cwd = match cwd.parent() {
            Some(parent) => parent.into(),
            None => {
                return Err(anyhow!(
                    "Cannot get repository root - this is not a git/svnc repo"
                ));
            }
        }
    }

    Ok(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Expander {
        Expander {
            config: Config::default(),
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
    fn selector_parsing() {
        assert_eq!(
            Expander::parse_selectors("1,1,l,a,1,33").unwrap().selectors,
            vec![
                Selector::FromFront(0),
                Selector::FromFront(0),
                Selector::FromBack(0),
                Selector::All,
                Selector::FromFront(0),
                Selector::FromFront(32),
            ]
        );
    }

    #[test]
    fn pattern_parsing() {
        let res = Expander::parse_pattern("@fish").unwrap();
        assert_eq!(res, (false, ".", "fish", None));

        let res = Expander::parse_pattern("@fish^tail").unwrap();
        assert_eq!(res, (false, ".", "fish", Some("tail")));

        let res = Expander::parse_pattern("@%head/**/fish^tail").unwrap();
        assert_eq!(res, (true, "head", "fish", Some("tail")));

        let res = Expander::parse_pattern("@/**/fish").unwrap();
        assert_eq!(res, (false, "/", "fish", None));

        let res = Expander::parse_pattern("@//**/fish").unwrap();
        assert_eq!(res, (false, "/", "fish", None));

        let res = Expander::parse_pattern("@./**/fish").unwrap();
        assert_eq!(res, (false, ".", "fish", None));

        let res = Expander::parse_pattern("@head/**/fish/**/tail").unwrap();
        assert_eq!(res, (false, "head", "fish/**/tail", None));

        let res = Expander::parse_pattern("@head/**/").unwrap();
        assert_eq!(res, (false, "head", "*/", None));
    }

    // '/' implies matching only directories
    #[test]
    fn imply_directory_matching() {
        let exp = setup();
        let arguments = vec!["@fo*/^a".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded.first().unwrap(), "./tests/foobar/");
    }

    #[test]
    fn transform_file_to_parent() {
        let mut exp = setup();
        exp.config.transform_files_to_dirs = true;
        let arguments = vec!["@src/*.rs^1".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded.first().unwrap(), "./src");
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
    fn search() {
        let exp = setup();

        let arguments = vec!["@*.rs^/nothingmatchesthis".to_string()];
        let expanded = exp.expand_arguments(&arguments).unwrap();
        assert_eq!(expanded.len(), 0);
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
        assert!(!expanded.is_empty());
    }
}
