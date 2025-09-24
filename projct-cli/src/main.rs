use clap::Parser;
use projct_core::{Args, ProjectTreeGenerator};

fn main() {
    let args = Args::parse();
    let command_or_path = args.command_or_path.clone();
    let path = args.path.clone();
    let (command, effective_path) = match (command_or_path, path) {
        (Some(cop), Some(p)) => (Some(cop), p),
        (Some(cop), None) if cop == "init" => (Some(cop), ".".to_string()),
        (Some(cop), None) => (None, cop),
        (None, Some(p)) => (None, p),
        (None, None) => (None, ".".to_string()),
    };

    if let Some(cmd) = command {
        if cmd == "init" {
            projct_core::config::Config::create_config(&args.config);
            return;
        }
    }

    let config = projct_core::config::Config::new(&args.config, &args, effective_path);
    let generator = ProjectTreeGenerator::new(config);
    generator.generate();
}
