# Code Auditor 🔍

AI-powered GitHub repository analyzer that uses **tool-calling agents** with [Ollama](https://ollama.ai) to analyze source code and generate detailed Markdown audit reports.

## ✨ What's New: Agentic Tool-Calling Architecture

Instead of sending entire files to the LLM, Code Auditor now uses an **agentic approach** where the LLM:

1. 🔍 **Explores** - Uses `list_files` to discover the project structure
2. 📖 **Reads** - Calls `read_file` to examine specific files
3. 🔎 **Searches** - Uses `search_code` to find patterns
4. 🐛 **Reports** - Calls `report_issue` for each problem found
5. ✅ **Finishes** - Calls `finish_analysis` when done

This is similar to how Claude or GPT-4 use tools to interact with codebases!

## Features

- 🤖 **Agentic Analysis**: LLM autonomously explores and analyzes the codebase
- 🛠️ **Tool Calling**: Uses Ollama's tool-calling API for structured interactions
- 🔒 **Security Scanning**: Identifies potential security vulnerabilities
- 🐛 **Bug Detection**: Finds logic errors, null pointer risks, race conditions
- ⚡ **Performance Issues**: Detects inefficient algorithms and blocking I/O
- 📊 **Comprehensive Reports**: Generates detailed Markdown reports with line numbers

## Prerequisites

1. **Rust** (latest stable): [Install Rust](https://rustup.rs/)
2. **LLM Backend** (choose one):
   - **Ollama** (recommended): [Install Ollama](https://ollama.ai/download)
   - **llama.cpp server**: [Build from source](https://github.com/ggerganov/llama.cpp)

3. **A model that supports tool calling**:
   ```bash
   # For Ollama:
   ollama pull llama3.2:latest     # Fast, good tool support
   ollama pull qwen3-coder:480b-cloud  # Cloud model, excellent
   
   # For llama.cpp:
   # Download a GGUF model and run:
   ./llama-server -m model.gguf --port 8080
   ```

> ⚠️ **Important**: The model MUST support tool/function calling.

## Installation

```bash
# Clone and build
git clone https://github.com/aarambh-darshan/code-auditor.git
cd code-auditor
cargo build --release

# Install system-wide (optional)
cargo install --path .
```

## Usage

### Basic Usage

```bash
# Analyze a GitHub repository
code-auditor --repo https://github.com/owner/repo.git

# Use a specific model (must support tool calling!)
code-auditor --repo https://github.com/owner/repo.git --model llama3.2:latest

# Analyze a local directory
code-auditor --repo local --local ./my-project
```

### All Options

```
code-auditor [OPTIONS] --repo <URL>

Options:
  -r, --repo <URL>           GitHub repository URL to analyze (required)
  -m, --model <NAME>         Ollama model name [default: deepseek-coder:33b]
  -o, --output <FILE>        Output file path [default: code_audit_report.md]
      --max-files <COUNT>    Maximum files to analyze [default: 100]
      --ollama-url <URL>     Ollama API endpoint [default: http://localhost:11434]
  -c, --config <FILE>        Path to configuration file
  -v, --verbose              Enable verbose logging
  -q, --quiet                Quiet mode (minimal output)
  -b, --branch <BRANCH>      Specific branch to analyze
      --local <DIR>          Analyze a local directory instead of cloning
      --format <FORMAT>      Output format: markdown, json [default: markdown]
  -h, --help                 Print help
  -V, --version              Print version
```

## How It Works (Agentic Architecture)

```
┌─────────────────────────────────────────────────────────────┐
│                      Code Auditor                           │
├─────────────────────────────────────────────────────────────┤
│  1. Clone Repository                                        │
│     └──> /tmp/repo_clone                                   │
│                                                             │
│  2. Initialize Agent with Tools                            │
│     ├── list_files(directory)   - Explore structure        │
│     ├── read_file(path)         - Read file contents       │
│     ├── search_code(pattern)    - Search for patterns      │
│     ├── get_file_info(path)     - Get file metadata        │
│     ├── report_issue(...)       - Report found issues      │
│     └── finish_analysis()       - Signal completion        │
│                                                             │
│  3. Agent Loop                                              │
│     ┌─────────────────────────────────────────────┐        │
│     │  LLM decides which tool to call             │        │
│     │     ↓                                       │        │
│     │  Tool executes, returns result              │        │
│     │     ↓                                       │        │
│     │  LLM processes result, may call more tools  │        │
│     │     ↓                                       │        │
│     │  Repeat until finish_analysis() called      │        │
│     └─────────────────────────────────────────────┘        │
│                                                             │
│  4. Generate Report from collected issues                  │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

Create `.code-auditor.toml` in your project:

```toml
[general]
output = "code_audit_report.md"
verbose = false

[model]
# LLM provider: "ollama" or "llamacpp"
provider = "ollama"
name = "llama3.2:latest"

# API endpoints
ollama_url = "http://localhost:11434"
llamacpp_url = "http://localhost:8080"

temperature = 0.1
timeout_seconds = 300

[scanner]
max_files = 100
extensions = ["rs", "py", "js", "ts", "go", "java", "c", "cpp"]
excludes = [".git", "target", "node_modules", "vendor"]
```

## Architecture

```
code-auditor/
├── src/
│   ├── main.rs              # CLI entry and workflow
│   ├── cli.rs               # Argument parsing
│   ├── config.rs            # Configuration handling
│   ├── models.rs            # Data structures
│   ├── repo/
│   │   └── cloner.rs        # Git repository cloning
│   ├── agent/
│   │   ├── tools.rs         # Tool definitions (NEW!)
│   │   ├── agent_loop.rs    # Agentic loop (NEW!)
│   │   ├── client.rs        # Ollama API client
│   │   └── prompts.rs       # LLM prompts
│   ├── analysis/
│   │   ├── orchestrator.rs  # Analysis coordination
│   │   └── aggregator.rs    # Issue aggregation
│   └── report/
│       └── generator.rs     # Report generation
├── Cargo.toml
└── .code-auditor.toml
```

## Troubleshooting

### "Cannot connect to Ollama"
- Ensure Ollama is running: `ollama serve`
- Check the URL: default is `http://localhost:11434`

### "Tool calling not working"
- Make sure you're using a model that supports tool calling
- Try: `llama3.2:latest`, `llama3.1:latest`, or `mistral:latest`
- Models like `deepseek-coder` may NOT support tools

### Analysis takes too long
- The agent may be exploring many files
- Use a faster model: `llama3.2:latest`
- Check Ollama logs for issues

## License

MIT License

## Acknowledgments

- [Ollama](https://ollama.ai) - Local LLM runtime with tool calling support
- [git2](https://github.com/rust-lang/git2-rs) - Git bindings for Rust
