use glob;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct GitPattern {
    pattern: String,
    is_negative: bool,
    is_directory_only: bool,
    is_absolute: bool,
}

#[derive(Clone)]
pub struct GitignoreParser {
    patterns: Vec<GitPattern>,
    gitignore_dir: String,
}

impl GitignoreParser {
    pub fn new(gitignore_path: Option<&Path>) -> Self {
        let mut patterns = vec![];
        patterns.push(GitPattern {
            pattern: ".git".to_string(),
            is_negative: false,
            is_directory_only: true,
            is_absolute: false,
        });
        patterns.push(GitPattern {
            pattern: ".gitattributes".to_string(),
            is_negative: false,
            is_directory_only: false,
            is_absolute: false,
        });
        patterns.push(GitPattern {
            pattern: ".gitignore".to_string(),
            is_negative: false,
            is_directory_only: false,
            is_absolute: false,
        });

        let gitignore_dir = gitignore_path
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut parser = GitignoreParser {
            patterns,
            gitignore_dir,
        };
        if let Some(path) = gitignore_path {
            if path.exists() {
                parser.load_patterns(path);
            }
        }
        parser
    }

    fn load_patterns(&mut self, gitignore_path: &Path) {
        let file = match File::open(gitignore_path) {
            Ok(f) => f,
            Err(e) => {
                println!("[Warning: Cannot read {}: {}]", gitignore_path.display(), e);
                return;
            }
        };
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l.trim().to_string(),
                Err(_) => continue,
            };
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(pattern) = self.parse_pattern(&line) {
                self.patterns.push(pattern);
            }
        }
    }

    fn parse_pattern(&self, pattern_line: &str) -> Option<GitPattern> {
        let mut pattern_line = pattern_line.replace("\\ ", " ");
        let is_negative = pattern_line.starts_with('!');
        if is_negative {
            pattern_line = pattern_line[1..].to_string();
        }
        let is_directory_only = pattern_line.ends_with('/');
        if is_directory_only {
            pattern_line = pattern_line[..pattern_line.len() - 1].to_string();
        }
        let is_absolute = pattern_line.starts_with('/');
        if is_absolute {
            pattern_line = pattern_line[1..].to_string();
        }
        let re = Regex::new(r"([?\[\]])").unwrap();
        let pattern_line = re.replace_all(&pattern_line, r"[$1]").to_string();
        Some(GitPattern {
            pattern: pattern_line,
            is_negative,
            is_directory_only,
            is_absolute,
        })
    }

    pub fn should_ignore(&self, path: &Path, is_directory: bool, parent_ignored: bool) -> bool {
        if self.patterns.is_empty() {
            return parent_ignored;
        }
        let gitignore_dir = Path::new(&self.gitignore_dir);
        let rel_path = match path.strip_prefix(gitignore_dir) {
            Ok(r) => r.to_path_buf(),
            Err(_) => return parent_ignored,
        };
        if rel_path.starts_with("..") {
            return parent_ignored;
        }
        let mut match_path = rel_path.to_string_lossy().to_string();
        if is_directory {
            match_path.push('/');
        }
        let mut result = parent_ignored;
        let mut last_negative_match = false;
        for pattern_info in &self.patterns {
            if pattern_info.is_directory_only && !is_directory {
                continue;
            }
            let target_path = if pattern_info.is_absolute {
                &match_path
            } else {
                &match_path
            };
            if self.matches_pattern(target_path, &pattern_info.pattern, pattern_info.is_absolute) {
                if pattern_info.is_negative {
                    last_negative_match = true;
                    result = false;
                } else {
                    result = true;
                    last_negative_match = false;
                }
            }
        }
        result && !last_negative_match
    }

    fn matches_pattern(&self, path: &str, pattern: &str, is_absolute: bool) -> bool {
        if pattern == "**" {
            return true;
        }
        let pattern = pattern.replace("**", "*");
        let glob_pattern = match glob::Pattern::new(&pattern) {
            Ok(p) => p,
            Err(_) => return false,
        };
        if is_absolute {
            if glob_pattern.matches(path) {
                return true;
            }
            let alt_pattern = format!("*/{}", pattern);
            let alt_glob = match glob::Pattern::new(&alt_pattern) {
                Ok(p) => p,
                Err(_) => return false,
            };
            alt_glob.matches(path)
        } else {
            path.split('/').any(|segment| glob_pattern.matches(segment))
        }
    }
}

pub struct HierarchicalGitignoreManager {
    start_path: PathBuf,
    parsers_by_dir: HashMap<PathBuf, Vec<GitignoreParser>>,
}

impl HierarchicalGitignoreManager {
    pub fn new(start_path: &Path) -> Self {
        let mut manager = HierarchicalGitignoreManager {
            start_path: start_path.to_path_buf(),
            parsers_by_dir: HashMap::new(),
        };
        manager.load_all_gitignores();
        manager
    }

    fn load_all_gitignores(&mut self) {
        for entry in WalkDir::new(&self.start_path) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    println!("[Warning: {}]", e);
                    continue;
                }
            };
            if entry.file_name() == ".gitignore" {
                let parser = GitignoreParser::new(Some(entry.path()));
                let dir = entry.path().parent().unwrap().to_path_buf();
                self.parsers_by_dir.entry(dir).or_default().push(parser);
            }
        }
    }

    fn find_relevant_parsers(&self, path: &Path) -> Vec<GitignoreParser> {
        let mut relevant = vec![];
        let mut current = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap().to_path_buf()
        };
        loop {
            if let Some(parsers) = self.parsers_by_dir.get(&current) {
                relevant.extend_from_slice(parsers);
            }
            if let Some(parent) = current.parent() {
                if parent == current {
                    break;
                }
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        relevant
    }

    pub fn should_ignore(&self, path: &Path, is_directory: bool) -> bool {
        let mut relevant_parsers = self.find_relevant_parsers(path);
        if relevant_parsers.is_empty() {
            return false;
        }
        relevant_parsers.sort_by_key(|p| p.gitignore_dir.len());
        let mut ignored = false;
        let mut last_negative_override = false;
        for parser in relevant_parsers {
            let current_ignored = parser.should_ignore(path, is_directory, ignored);
            if ignored && !current_ignored {
                last_negative_override = true;
                ignored = false;
            } else {
                ignored = current_ignored;
                last_negative_override = false;
            }
        }
        ignored && !last_negative_override
    }
}
