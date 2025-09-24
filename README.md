# Projct

A fast, configurable directory tree generator with file content extraction

## Features

- Generate hierarchical directory trees
- Extract and display file contents
- Configurable via TOML file or CLI arguments

## Installation

- **Pre-built binaries**: Download from [Releases](https://github.com/user-with-username/projct/releases)
- **Build from source**:
```bash
git clone https://github.com/user-with-username/projct
cd projct
cargo install --path .
```

## Quick Start

```bash
# Generate tree with default settings
projct

# Create sample config file
projct init

# Generate tree with custom output
projct -o my_output.txt --line-numbers
```

## Configuration

Create `projct.toml`:

```toml
[general]
path = "."
use_gitignore = true
max_depth = 3

[output]
filename = "output.txt"
max_file_size = 50000
show_line_numbers = true

[filters]
include_patterns = ["*.rs", "*.toml"]
exclude_patterns = ["target/*", "*.log"]
```

## License

MIT