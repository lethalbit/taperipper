// SPDX-License-Identifier: BSD-3-Clause

use clap::Command;
use tracing::error;
use tracing_subscriber::{
    self, Layer,
    filter::{EnvFilter, LevelFilter},
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

mod commands;
mod paths;
mod utils;

fn main() {
    tracing_subscriber::registry()
        .with(
            fmt::layer().with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .with_env_var("TAPERIPPER_XTASK_LOG_LEVEL")
                    .from_env_lossy()
                    .add_directive("goblin=info".parse().unwrap()),
            ),
        )
        .init();

    let args = Command::new("taperipper-xtask")
        .subcommands(commands::init())
        .get_matches();

    if let Some(cmd) = args.subcommand() {
        if let Some(cmd_exec) = commands::exec(cmd.0) {
            if let Err(err) = cmd_exec(cmd.1) {
                error!("Command Failed!");
                error!("{err}");
            }
        } else {
            error!("Unimplemented subcommand '{}'", cmd.0)
        }
    } else {
        error!("Unable to find command!");
    }
}
