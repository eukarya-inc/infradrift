mod cli;
mod config;
mod drift;
mod filter;
mod output;
mod plan;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Command};
use config::Config;
use drift::detector::detect_drift;
use filter::engine::Filters;
use std::path::Path;
use std::process;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {:#}", e);
        process::exit(2);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    if let Command::Completions { shell } = cli.command {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "infradrift", &mut std::io::stdout());
        return Ok(());
    }

    if let Command::Validate { config } = &cli.command {
        return run_validate(config);
    }

    let (plan, common) = match cli.command {
        Command::Scan {
            dir,
            tofu,
            plan_args,
            common,
        } => {
            let plan = plan::executor::execute_plan(&dir, tofu, &plan_args)?;
            (plan, common)
        }
        Command::Completions { .. } | Command::Validate { .. } => unreachable!(),
        Command::Parse {
            file,
            binary,
            tofu,
            common,
        } => {
            let plan = plan::parser::parse_plan_file(&file, binary, tofu)?;
            (plan, common)
        }
    };

    // Load config
    let config = Config::load(&common.config)?;

    // Detect drift
    let mut report = detect_drift(&plan, &config);

    // Apply CLI filters
    let filters = Filters {
        resource_types: common.resource_types,
        resource_names: common.resource_names,
        attributes: common.attributes,
    };
    filters.apply(&mut report);

    // Output
    let mut stdout = std::io::stdout();
    output::render(&report, &common.format, common.no_color, &mut stdout)?;

    // Exit code: 1 if drift detected, 0 if clean
    if report.drifted_resources.is_empty() {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn run_validate(config_path: &Path) -> anyhow::Result<()> {
    match Config::load_and_validate(config_path) {
        Ok(warnings) => {
            if warnings.is_empty() {
                println!("✓ {} is valid", config_path.display());
            } else {
                println!(
                    "⚠ {} has {} warning(s):",
                    config_path.display(),
                    warnings.len()
                );
                for w in &warnings {
                    println!("  - {}", w);
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("✗ {} is invalid: {:#}", config_path.display(), e);
            process::exit(2);
        }
    }
}
