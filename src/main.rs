//! Code Auditor - AI-powered GitHub Repository Analyzer
//!
//! A CLI tool that uses Ollama with tool-calling to analyze
//! source code repositories and generate detailed audit reports.

mod agent;
mod analysis;
mod cli;
mod config;
mod models;
mod repo;
mod report;
mod scanner;

use anyhow::{Context, Result};
use chrono::Utc;
use cli::{Args, OutputFormat};
use config::Config;
use models::{AnalyzedFile, Issue, IssueSummary, Report, ReportMetadata, Severity};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, error, info, warn};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse_args();

    // Validate arguments
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Initialize logging
    init_logging(&args);

    info!("Code Auditor v{}", env!("CARGO_PKG_VERSION"));
    debug!("Arguments: {:?}", args);

    // Run the audit
    match run_audit(args).await {
        Ok(output_path) => {
            println!(
                "\n✅ Audit complete! Report saved to: {}",
                output_path.display()
            );
            Ok(())
        }
        Err(e) => {
            error!("Audit failed: {}", e);
            eprintln!("\n❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Initialize logging based on verbosity settings.
fn init_logging(args: &Args) {
    let level = args.log_level();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

/// Run the complete audit workflow.
async fn run_audit(args: Args) -> Result<PathBuf> {
    let start_time = Instant::now();

    // Load configuration
    let mut config = load_config(&args)?;
    config.merge_with_args(&args);

    // Step 1: Get the repository
    println!("📥 Cloning repository: {}", args.repo);
    let repo_path = get_repository(&args).await?;
    info!("Repository at: {}", repo_path.display());

    // Try to load config from repository
    if let Ok(Some(repo_config)) = Config::load_from_repo(&repo_path) {
        info!("Found .code-auditor.toml in repository");
        config = repo_config;
        config.merge_with_args(&args);
    }

    // Step 2: Initialize the agent
    println!("🤖 Initializing AI agent...");
    println!("   Model: {}", config.model.name);
    println!("   Ollama: {}", config.model.ollama_url);
    println!(
        "   Mode: {}",
        if config.model.single_call_mode {
            "Single-call (efficient)"
        } else {
            "Tool-calling (agentic)"
        }
    );

    let agent_config = agent::AgentConfig {
        ollama_url: config.model.ollama_url.clone(),
        model_name: config.model.name.clone(),
        temperature: config.model.temperature,
        max_iterations: 50,
        timeout_seconds: config.model.timeout_seconds,
        single_call_mode: config.model.single_call_mode,
        max_context_messages: 10, // Sliding window to prevent context overflow
    };

    let mut agent = agent::CodeAnalysisAgent::new(agent_config, repo_path.clone());

    // Step 3: Run the agentic analysis
    println!("\n🔬 Running code analysis...");
    if config.model.single_call_mode {
        println!("   Reading all files and sending in ONE API call...\n");
    } else {
        println!("   The AI agent will explore the repository using tools...\n");
    }

    let reported_issues = agent.run_analysis().await?;

    // Step 4: Convert reported issues to our Issue format
    let issues: Vec<Issue> = reported_issues
        .into_iter()
        .map(|ri| Issue {
            file_path: ri.file_path,
            start_line: ri.line_number,
            end_line: None,
            severity: match ri.severity.to_lowercase().as_str() {
                "critical" => Severity::Critical,
                "high" => Severity::High,
                "medium" => Severity::Medium,
                _ => Severity::Low,
            },
            category: ri.category,
            title: ri.title,
            description: ri.description,
            suggestion: ri.suggestion,
            code_snippet: None,
        })
        .collect();

    // Step 5: Build the report
    println!("\n📝 Generating report...");

    let duration = start_time.elapsed().as_secs_f64();
    let summary = IssueSummary::from_issues(&issues);

    // Group issues by file
    let mut files_map: std::collections::HashMap<String, Vec<Issue>> =
        std::collections::HashMap::new();
    for issue in issues {
        files_map
            .entry(issue.file_path.clone())
            .or_default()
            .push(issue);
    }

    let analyzed_files: Vec<AnalyzedFile> = files_map
        .into_iter()
        .map(|(path, file_issues)| AnalyzedFile {
            path,
            language: "Unknown".to_string(),
            line_count: 0,
            issues: file_issues,
            analysis_successful: true,
            error: None,
        })
        .collect();

    let metadata = ReportMetadata {
        repo_url: args.repo.clone(),
        analysis_date: Utc::now(),
        model_used: config.model.name.clone(),
        files_analyzed: analyzed_files.len(),
        files_failed: 0,
        total_issues: summary.total,
        duration_seconds: duration,
    };

    let report = Report {
        metadata,
        project_overview: "Analysis performed by AI agent with tool-calling capabilities."
            .to_string(),
        files: analyzed_files,
        summary: summary.clone(),
        recommendations: vec![
            "Review all reported issues and prioritize by severity.".to_string(),
            "Address critical and high severity issues first.".to_string(),
        ],
    };

    // Step 6: Generate and save the report
    let output = match args.format {
        OutputFormat::Json => report::generate_json_report(&report)?,
        OutputFormat::Markdown => report::generate_markdown_report(&report),
    };

    std::fs::write(&args.output, &output)
        .with_context(|| format!("Failed to write report to {}", args.output.display()))?;

    // Print summary
    println!("\n📊 Analysis Summary:");
    println!("   Files with issues: {}", report.files.len());
    println!("   Total issues: {}", summary.total);
    println!(
        "   - 🔴 Critical: {} | 🟠 High: {} | 🟡 Medium: {} | 🟢 Low: {}",
        summary.critical, summary.high, summary.medium, summary.low
    );
    println!("   Duration: {:.1}s", duration);

    Ok(args.output)
}

/// Load configuration from file or use defaults.
fn load_config(args: &Args) -> Result<Config> {
    // Try explicit config path
    if let Some(ref config_path) = args.config {
        info!("Loading config from: {}", config_path.display());
        return Config::load(config_path);
    }

    // Try default location
    match Config::load_default() {
        Ok(Some(config)) => {
            info!("Loaded default config from .code-auditor.toml");
            Ok(config)
        }
        Ok(None) => {
            debug!("No config file found, using defaults");
            Ok(Config::default())
        }
        Err(e) => {
            warn!("Failed to load config: {}", e);
            Ok(Config::default())
        }
    }
}

/// Get the repository path (clone if needed).
async fn get_repository(args: &Args) -> Result<PathBuf> {
    // Use local directory if specified
    if let Some(ref local) = args.local {
        info!("Using local directory: {}", local.display());
        return Ok(local.clone());
    }

    // Clone the repository
    info!("Cloning repository: {}", args.repo);

    let clone_options = repo::CloneOptions {
        branch: args.branch.clone(),
        depth: Some(1), // Shallow clone
        show_progress: !args.quiet,
        target_dir: None,
    };

    let clone_result = repo::clone_repository(&args.repo, clone_options)?;
    Ok(clone_result.into_path())
}
