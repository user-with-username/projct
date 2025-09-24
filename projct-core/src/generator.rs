use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::file_utils::FileUtils;
use crate::gitignore::HierarchicalGitignoreManager;

pub struct OutputWriter<'a> {
    pub config: &'a Config,
    pub gitignore_manager: Option<&'a HierarchicalGitignoreManager>,
}

impl<'a> OutputWriter<'a> {
    pub fn write_tree_and_get_files(
        &self,
        start_path: &Path,
        output_file: &mut dyn Write,
        depth: u32,
        prefix: &str,
    ) -> Vec<PathBuf> {
        if let Some(md) = self.config.general.max_depth {
            if depth > md {
                return vec![];
            }
        }

        let is_directory = start_path.is_dir();
        let show_ignored = self.config.general.show_ignored;
        let show_binary = self.config.general.show_binary;
        let output_filename = &self.config.output.filename;

        if start_path
            .file_name()
            .map_or(false, |name| name == output_filename.as_str())
        {
            return vec![];
        }

        let mut is_ignored = false;
        if let Some(gm) = self.gitignore_manager {
            if depth > 0 {
                is_ignored = gm.should_ignore(start_path, is_directory);
            }
        }

        if is_ignored && !show_ignored {
            return vec![];
        }

        if !is_directory {
            if !FileUtils::is_text_file(start_path) {
                if !show_binary {
                    return vec![];
                }
            }
        }

        if !is_directory {
            return vec![start_path.to_path_buf()];
        }

        let mut collected_files = vec![];
        let entries = match std::fs::read_dir(start_path) {
            Ok(e) => e,
            Err(_) => {
                let _ = writeln!(output_file, "{}└── [Permission Denied]", prefix);
                return collected_files;
            }
        };

        let mut items: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();

        items.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });

        let num_items = items.len();
        for (i, item_path) in items.iter().enumerate() {
            let is_last = i == num_items - 1;
            let connector = if is_last { "└── " } else { "├── " };
            let item_is_dir = item_path.is_dir();

            let item_is_ignored = self
                .gitignore_manager
                .as_ref()
                .map_or(false, |gm| gm.should_ignore(item_path, item_is_dir));
            if item_is_ignored && !show_ignored {
                continue;
            }

            if !item_is_dir && !FileUtils::is_text_file(item_path) && !show_binary {
                continue;
            }
            if item_path
                .file_name()
                .map_or(false, |name| name == output_filename.as_str())
            {
                continue;
            }

            let display_name = item_path.file_name().unwrap().to_string_lossy();
            let _ = writeln!(
                output_file,
                "{}{}{}{}",
                prefix,
                connector,
                display_name,
                if item_is_dir { "/" } else { "" }
            );

            if item_is_dir {
                let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
                collected_files.extend(self.write_tree_and_get_files(
                    item_path,
                    output_file,
                    depth + 1,
                    &new_prefix,
                ));
            } else {
                collected_files.push(item_path.clone());
            }
        }

        collected_files
    }

    pub fn write_file_contents(
        &self,
        file_list: &[PathBuf],
        output_file: &mut dyn Write,
        start_path: &Path,
    ) {
        if file_list.is_empty() {
            return;
        }
        let max_file_size = self.config.output.max_file_size;
        let show_line_numbers = self.config.output.show_line_numbers;
        for file_path in file_list {
            let rel_path = file_path
                .strip_prefix(start_path)
                .unwrap_or(file_path)
                .to_string_lossy();
            let header = format!("\n{}:\n", rel_path);
            let _ = output_file.write_all(header.as_bytes());
            let file_size = match file_path.metadata() {
                Ok(m) => m.len(),
                Err(_) => 0,
            };
            if max_file_size > 0 && file_size > max_file_size {
                let msg = format!("[File is too big to show ({} bytes)]\n", file_size);
                let _ = output_file.write_all(msg.as_bytes());
                continue;
            }
            let mut file = match File::open(file_path) {
                Ok(f) => f,
                Err(e) => {
                    let msg = format!("[Cannot read {}: {}]\n", rel_path, e);
                    let _ = output_file.write_all(msg.as_bytes());
                    continue;
                }
            };
            let mut content = String::new();
            if file.read_to_string(&mut content).is_err() {
                let msg = format!("[Cannot read {}: invalid UTF-8]\n", rel_path);
                let _ = output_file.write_all(msg.as_bytes());
                continue;
            }
            if content.trim().is_empty() {
                let _ = output_file.write_all(b"[Empty]\n");
            } else {
                let lines = content.lines().enumerate();
                for (line_num, line) in lines {
                    let out_line = if show_line_numbers {
                        format!("{:4}: {}\n", line_num + 1, line)
                    } else {
                        format!("{}\n", line)
                    };
                    let _ = output_file.write_all(out_line.as_bytes());
                }
            }
        }
    }
}

pub struct ProjectTreeGenerator {
    pub config: Config,
    pub gitignore_manager: Option<HierarchicalGitignoreManager>,
}

impl ProjectTreeGenerator {
    pub fn new(config: Config) -> Self {
        let gitignore_manager = if config.general.use_gitignore {
            Some(HierarchicalGitignoreManager::new(Path::new(
                &config.general.path,
            )))
        } else {
            None
        };
        Self {
            config,
            gitignore_manager,
        }
    }

    pub fn generate(&self) {
        let output_filename = self.config.output.filename.clone();
        let start_path = Path::new(&self.config.general.path);
        let mut output_file = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&output_filename)
        {
            Ok(f) => f,
            Err(e) => {
                println!("Cannot open output file: {}", e);
                return;
            }
        };
        let output_writer = OutputWriter {
            config: &self.config,
            gitignore_manager: self.gitignore_manager.as_ref(),
        };

        let root_display_name = start_path
            .canonicalize()
            .unwrap_or(start_path.to_path_buf())
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("."))
            .to_string_lossy()
            .to_string();
        let _ = writeln!(&mut output_file, "{}/", root_display_name);

        let mut file_list =
            output_writer.write_tree_and_get_files(start_path, &mut output_file, 0, "");

        file_list = self.filter_file_list(file_list);

        output_writer.write_file_contents(&file_list, &mut output_file, start_path);
    }

    fn filter_file_list(&self, mut file_list: Vec<PathBuf>) -> Vec<PathBuf> {
        let include_patterns = &self.config.filters.include_patterns;
        let exclude_patterns = &self.config.filters.exclude_patterns;
        if !include_patterns.is_empty() {
            file_list.retain(|f| {
                let name = f.file_name().unwrap_or_default().to_string_lossy();
                include_patterns.iter().any(|p| {
                    glob::Pattern::new(p)
                        .ok()
                        .map_or(false, |gp| gp.matches(&name))
                })
            });
        }
        if !exclude_patterns.is_empty() {
            file_list.retain(|f| {
                let name = f.file_name().unwrap_or_default().to_string_lossy();
                !exclude_patterns.iter().any(|p| {
                    glob::Pattern::new(p)
                        .ok()
                        .map_or(false, |gp| gp.matches(&name))
                })
            });
        }
        file_list
    }
}
