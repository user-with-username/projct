use clap::Parser;
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const CONFIG: &str = r#"[general]
path = "."

[output]
filename = "output.txt"
"#;

#[derive(Deserialize, Debug, Default)]
struct RawGeneral {
    path: Option<String>,
    max_depth: Option<u32>,
    use_gitignore: Option<bool>,
    show_ignored: Option<bool>,
    show_binary: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
struct RawOutput {
    filename: Option<String>,
    max_file_size: Option<u64>,
    show_line_numbers: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
struct RawFilters {
    include_patterns: Option<Vec<String>>,
    exclude_patterns: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Default)]
struct RawConfig {
    general: Option<RawGeneral>,
    output: Option<RawOutput>,
    filters: Option<RawFilters>,
}

#[derive(Clone, Debug)]
pub struct General {
    pub path: String,
    pub max_depth: Option<u32>,
    pub use_gitignore: bool,
    pub show_ignored: bool,
    pub show_binary: bool,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub filename: String,
    pub max_file_size: u64,
    pub show_line_numbers: bool,
}

#[derive(Clone, Debug)]
pub struct Filters {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub general: General,
    pub output: Output,
    pub filters: Filters,
}

#[derive(Parser, Clone)]
#[command(about = "Generate directory tree with file contents")]
pub struct Args {
    #[arg(help = "Command to execute (use 'init' to create config) or starting path")]
    pub command_or_path: Option<String>,

    #[arg(help = "Starting path if command is provided")]
    pub path: Option<String>,

    #[arg(long, help = "Maximum depth to traverse")]
    pub max_depth: Option<u32>,

    #[arg(long, default_value_t = 100000, help = "Maximum file size to display")]
    pub max_size: u64,

    #[arg(long, help = "Show line numbers")]
    pub line_numbers: bool,

    #[arg(long, help = "Ignore .gitignore files")]
    pub no_gitignore: bool,

    #[arg(long, help = "Show ignored files")]
    pub show_ignored: bool,

    #[arg(long, help = "Show binary files")]
    pub show_binary: bool,

    #[arg(short = 'o', long, help = "Output filename")]
    pub output: Option<String>,

    #[arg(
        short = 'c',
        long,
        default_value = "projct.toml",
        help = "Config file path"
    )]
    pub config: String,
}

impl Config {
    pub fn new(config_path: &str, args: &Args, effective_path: String) -> Self {
        let mut config = Self::load_config(config_path);

        config.general.path = effective_path;
        if let Some(md) = args.max_depth {
            config.general.max_depth = Some(md);
        }
        if args.no_gitignore {
            config.general.use_gitignore = false;
        }
        if args.show_ignored {
            config.general.show_ignored = true;
        }
        if args.show_binary {
            config.general.show_binary = true;
        }
        if let Some(o) = &args.output {
            config.output.filename = o.clone();
        }
        if args.max_size != 100000 {
            config.output.max_file_size = args.max_size;
        }
        if args.line_numbers {
            config.output.show_line_numbers = true;
        }

        config
    }

    fn load_config(config_path: &str) -> Self {
        let default_config = Self::default_config();

        if !Path::new(config_path).exists() {
            return default_config;
        }

        let mut file = match File::open(config_path) {
            Ok(f) => f,
            Err(e) => {
                println!("Cannot load config: {}. Using defaults.", e);
                return default_config;
            }
        };

        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_err() {
            println!("Cannot read config. Using defaults.");
            return default_config;
        }

        let loaded_raw: RawConfig = match toml::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                println!("Cannot parse config: {}. Using defaults.", e);
                return default_config;
            }
        };

        let loaded_general = loaded_raw.general.unwrap_or_default();
        let loaded_output = loaded_raw.output.unwrap_or_default();
        let loaded_filters = loaded_raw.filters.unwrap_or_default();

        Config {
            general: General {
                path: loaded_general.path.unwrap_or(default_config.general.path),
                max_depth: loaded_general
                    .max_depth
                    .or(default_config.general.max_depth),
                use_gitignore: loaded_general
                    .use_gitignore
                    .unwrap_or(default_config.general.use_gitignore),
                show_ignored: loaded_general
                    .show_ignored
                    .unwrap_or(default_config.general.show_ignored),
                show_binary: loaded_general
                    .show_binary
                    .unwrap_or(default_config.general.show_binary),
            },
            output: Output {
                filename: loaded_output
                    .filename
                    .unwrap_or(default_config.output.filename),
                max_file_size: loaded_output
                    .max_file_size
                    .unwrap_or(default_config.output.max_file_size),
                show_line_numbers: loaded_output
                    .show_line_numbers
                    .unwrap_or(default_config.output.show_line_numbers),
            },
            filters: Filters {
                include_patterns: loaded_filters
                    .include_patterns
                    .unwrap_or(default_config.filters.include_patterns),
                exclude_patterns: loaded_filters
                    .exclude_patterns
                    .unwrap_or(default_config.filters.exclude_patterns),
            },
        }
    }

    fn default_config() -> Self {
        Config {
            general: General {
                path: ".".to_string(),
                max_depth: None,
                use_gitignore: true,
                show_ignored: false,
                show_binary: false,
            },
            output: Output {
                filename: "output.txt".to_string(),
                max_file_size: 100000,
                show_line_numbers: false,
            },
            filters: Filters {
                include_patterns: vec![],
                exclude_patterns: vec![],
            },
        }
    }

    pub fn create_config(config_path: &str) {
        let mut file = match File::create(config_path) {
            Ok(f) => f,
            Err(e) => {
                println!("Error creating config: {}", e);
                return;
            }
        };
        if file.write_all(CONFIG.as_bytes()).is_err() {
            println!("Error writing config.");
        } else {
        }
    }
}
