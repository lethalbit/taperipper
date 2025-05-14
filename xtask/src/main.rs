// SPDX-License-Identifier: BSD-3-Clause

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

    let args =
        crate::commands::setup_commands(clap::Command::new("taperipper-xtask")).get_matches();

    if let Err(err) = crate::commands::dispatch(args) {
        error!("Command Failed!");
        error!("{err}");
    }
}
