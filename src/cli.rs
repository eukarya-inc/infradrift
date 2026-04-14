use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "infradrift",
    version = env!("INFRADRIFT_VERSION"),
    about = "Detect infrastructure drift from Terraform/OpenTofu plans"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run terraform/tofu plan and detect drift
    Scan {
        /// Path to the Terraform/OpenTofu working directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,

        /// Use OpenTofu instead of Terraform
        #[arg(long)]
        tofu: bool,

        /// Additional arguments to pass to terraform/tofu plan
        #[arg(last = true)]
        plan_args: Vec<String>,

        #[command(flatten)]
        common: CommonArgs,
    },

    /// Validate an infradrift.toml configuration file
    Validate {
        /// Path to config file to validate
        #[arg(short, long, default_value = "infradrift.toml")]
        config: PathBuf,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Parse an existing plan file and detect drift
    Parse {
        /// Path to the plan file (JSON from `terraform show -json` or binary planfile)
        #[arg(short, long)]
        file: PathBuf,

        /// Force treating the file as a binary planfile (runs terraform show -json on it)
        #[arg(long)]
        binary: bool,

        /// Use OpenTofu instead of Terraform for converting binary planfiles
        #[arg(long)]
        tofu: bool,

        #[command(flatten)]
        common: CommonArgs,
    },
}

#[derive(Args, Clone)]
pub struct CommonArgs {
    /// Output format
    #[arg(short = 'o', long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,

    /// Filter by resource type (e.g., aws_instance). Repeatable.
    #[arg(long = "type", short = 't')]
    pub resource_types: Vec<String>,

    /// Filter by resource address (supports glob patterns, e.g., "module.vpc.*"). Repeatable.
    #[arg(long = "name", short = 'n')]
    pub resource_names: Vec<String>,

    /// Filter by changed attribute name (e.g., "tags"). Repeatable.
    #[arg(long = "attr", short = 'a')]
    pub attributes: Vec<String>,

    /// Suppress colored output
    #[arg(long)]
    pub no_color: bool,

    /// Path to config file with ignore rules
    #[arg(short, long, default_value = "infradrift.toml")]
    pub config: PathBuf,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
    Csv,
    Table,
    Hcl,
}
