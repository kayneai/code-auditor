//! Command-line interface argument parsing.
//!
//! This module handles all CLI argument parsing using clap,
//! including validation and default values.

use clap::Parser;
use std::path::PathBuf;

/// Code Auditor - AI-powered GitHub repository analyzer
///
/// Analyzes source code repositories using LLMs to identify bugs,
/// security issues, performance problems, and code quality concerns.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// GitHub repository URL to analyze
    ///
    /// Supports HTTPS URLs (e.g., https://github.com/owner/repo.git)
    #[arg(short, long, value_name = "URL")]
    pub repo: String,

    /// Ollama model to use for analysis
    ///
    /// Recommended models: deepseek-coder:33b, codellama:34b, qwen2.5-coder:32b
    #[arg(
        short,
        long,
        default_value = "deepseek-coder:33b",
        env = "CODE_AUDITOR_MODEL"
    )]
    pub model: String,

    /// Output file path for the Markdown report
    #[arg(
        short,
        long,
        default_value = "code_audit_report.md",
        value_name = "FILE"
    )]
    pub output: PathBuf,

    /// Maximum number of files to analyze
    ///
    /// Files are selected based on priority (main source files first)
    #[arg(long, default_value = "100", value_name = "COUNT")]
    pub max_files: usize,

    /// Ollama API endpoint URL
    #[arg(long, default_value = "http://localhost:11434", env = "OLLAMA_URL")]
    pub ollama_url: String,

    /// Path to configuration file
    ///
    /// If not specified, looks for .code-auditor.toml in the current directory
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging output
    #[arg(short, long)]
    pub verbose: bool,

    /// Run in quiet mode (minimal output)
    #[arg(short, long)]
    pub quiet: bool,

    /// Specific branch to analyze
    ///
    /// If not specified, uses the default branch
    #[arg(short, long, value_name = "BRANCH")]
    pub branch: Option<String>,

    /// File extensions to include (comma-separated)
    ///
    /// Example: --extensions rs,py,js
    #[arg(long, value_name = "EXTS", value_delimiter = ',')]
    pub extensions: Option<Vec<String>>,

    /// Patterns to exclude from analysis (comma-separated)
    ///
    /// Example: --exclude "test/*,vendor/*"
    #[arg(long, value_name = "PATTERNS", value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,

    /// Number of concurrent file analyses
    #[arg(long, default_value = "4", value_name = "NUM")]
    pub concurrency: usize,

    /// Skip cloning if directory already exists
    #[arg(long)]
    pub skip_clone: bool,

    /// Local directory to analyze instead of cloning
    #[arg(long, value_name = "DIR")]
    pub local: Option<PathBuf>,

    /// Output format (markdown, json)
    #[arg(long, default_value = "markdown", value_name = "FORMAT")]
    pub format: OutputFormat,

    /// Temperature for LLM responses (0.0 - 1.0)
    ///
    /// Lower values produce more consistent/deterministic output
    #[arg(long, default_value = "0.1")]
    pub temperature: f32,

    /// Maximum context window size for chunking large files
    #[arg(long, default_value = "4000", value_name = "LINES")]
    pub max_chunk_lines: usize,
}

/// Output format for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum OutputFormat {
    /// Markdown format (default)
    #[default]
    Markdown,
    /// JSON format
    Json,
}

impl Args {
    /// Parse command-line arguments.
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Validate the parsed arguments.
    pub fn validate(&self) -> Result<(), String> {
        // Validate repository URL format
        if !self.repo.starts_with("https://") && !self.repo.starts_with("git@") {
            if self.local.is_none() {
                return Err("Repository URL must start with 'https://' or 'git@'".to_string());
            }
        }

        // Validate Ollama URL format
        if !self.ollama_url.starts_with("http://") && !self.ollama_url.starts_with("https://") {
            return Err("Ollama URL must start with 'http://' or 'https://'".to_string());
        }

        // Validate temperature range
        if !(0.0..=1.0).contains(&self.temperature) {
            return Err("Temperature must be between 0.0 and 1.0".to_string());
        }

        // Validate concurrency
        if self.concurrency == 0 {
            return Err("Concurrency must be at least 1".to_string());
        }

        // Validate max files
        if self.max_files == 0 {
            return Err("Max files must be at least 1".to_string());
        }

        // Check for conflicting options
        if self.verbose && self.quiet {
            return Err("Cannot use both --verbose and --quiet".to_string());
        }

        // Validate local directory if provided
        if let Some(ref local_path) = self.local {
            if !local_path.exists() {
                return Err(format!(
                    "Local directory does not exist: {}",
                    local_path.display()
                ));
            }
            if !local_path.is_dir() {
                return Err(format!(
                    "Local path is not a directory: {}",
                    local_path.display()
                ));
            }
        }

        Ok(())
    }

    /// Returns the log level based on verbosity settings.
    pub fn log_level(&self) -> tracing::Level {
        if self.quiet {
            tracing::Level::ERROR
        } else if self.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        }
    }

    /// Returns the effective list of file extensions to analyze.
    #[allow(dead_code)]
    pub fn effective_extensions(&self) -> Vec<String> {
        self.extensions.clone().unwrap_or_else(|| {
            vec![
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h", "hpp", "cs",
                "rb", "php", "swift", "kt", "scala", "vue", "svelte",
            ]
            .into_iter()
            .map(String::from)
            .collect()
        })
    }

    /// Returns the effective list of exclude patterns.
    #[allow(dead_code)]
    pub fn effective_excludes(&self) -> Vec<String> {
        self.exclude.clone().unwrap_or_else(|| {
            vec![
                ".git",
                "target",
                "node_modules",
                "vendor",
                "dist",
                "build",
                "__pycache__",
                ".venv",
                "venv",
                ".idea",
                ".vscode",
                "*.min.js",
                "*.min.css",
                "package-lock.json",
                "Cargo.lock",
                "yarn.lock",
            ]
            .into_iter()
            .map(String::from)
            .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_extensions() {
        let args = Args {
            repo: "https://github.com/test/repo".to_string(),
            model: "test".to_string(),
            output: PathBuf::from("test.md"),
            max_files: 100,
            ollama_url: "http://localhost:11434".to_string(),
            config: None,
            verbose: false,
            quiet: false,
            branch: None,
            extensions: None,
            exclude: None,
            concurrency: 4,
            skip_clone: false,
            local: None,
            format: OutputFormat::Markdown,
            temperature: 0.1,
            max_chunk_lines: 4000,
        };

        let exts = args.effective_extensions();
        assert!(exts.contains(&"rs".to_string()));
        assert!(exts.contains(&"py".to_string()));
        assert!(exts.contains(&"js".to_string()));
    }

    #[test]
    fn test_validation_invalid_url() {
        let args = Args {
            repo: "invalid-url".to_string(),
            model: "test".to_string(),
            output: PathBuf::from("test.md"),
            max_files: 100,
            ollama_url: "http://localhost:11434".to_string(),
            config: None,
            verbose: false,
            quiet: false,
            branch: None,
            extensions: None,
            exclude: None,
            concurrency: 4,
            skip_clone: false,
            local: None,
            format: OutputFormat::Markdown,
            temperature: 0.1,
            max_chunk_lines: 4000,
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validation_conflicting_options() {
        let args = Args {
            repo: "https://github.com/test/repo".to_string(),
            model: "test".to_string(),
            output: PathBuf::from("test.md"),
            max_files: 100,
            ollama_url: "http://localhost:11434".to_string(),
            config: None,
            verbose: true,
            quiet: true, // Conflict!
            branch: None,
            extensions: None,
            exclude: None,
            concurrency: 4,
            skip_clone: false,
            local: None,
            format: OutputFormat::Markdown,
            temperature: 0.1,
            max_chunk_lines: 4000,
        };

        assert!(args.validate().is_err());
    }

    #[test]
    fn test_log_level() {
        let mut args = Args {
            repo: "https://github.com/test/repo".to_string(),
            model: "test".to_string(),
            output: PathBuf::from("test.md"),
            max_files: 100,
            ollama_url: "http://localhost:11434".to_string(),
            config: None,
            verbose: false,
            quiet: false,
            branch: None,
            extensions: None,
            exclude: None,
            concurrency: 4,
            skip_clone: false,
            local: None,
            format: OutputFormat::Markdown,
            temperature: 0.1,
            max_chunk_lines: 4000,
        };

        assert_eq!(args.log_level(), tracing::Level::INFO);

        args.verbose = true;
        assert_eq!(args.log_level(), tracing::Level::DEBUG);

        args.verbose = false;
        args.quiet = true;
        assert_eq!(args.log_level(), tracing::Level::ERROR);
    }
}
