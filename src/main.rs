use anyhow::Result;
use clap::Parser;

use tile_prune::cli::{Cli, Command};
use tile_prune::format::{plan_copy, plan_optimize, resolve_output_path};

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log);

    match cli.command {
        Command::Inspect(args) => {
            println!("inspect: input={}", args.input.display());
        }
        Command::Optimize(args) => {
            let decision = plan_optimize(
                &args.input,
                args.output.as_deref(),
                args.input_format.as_deref(),
                args.output_format.as_deref(),
            )?;
            let _output_path =
                resolve_output_path(&args.input, args.output.as_deref(), decision.output);
            println!("optimize: input={}", args.input.display());
        }
        Command::Simplify(args) => {
            println!("simplify: input={} z={} x={} y={}", args.input.display(), args.z, args.x, args.y);
        }
        Command::Copy(args) => {
            let decision = plan_copy(
                &args.input,
                args.output.as_deref(),
                args.input_format.as_deref(),
                args.output_format.as_deref(),
            )?;
            let _output_path =
                resolve_output_path(&args.input, args.output.as_deref(), decision.output);
            println!("copy: input={}", args.input.display());
        }
        Command::Verify(args) => {
            println!("verify: input={}", args.input.display());
        }
    }

    Ok(())
}

fn init_tracing(level: &str) {
    let filter = tracing_subscriber::EnvFilter::try_new(level).unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("info")
    });
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
