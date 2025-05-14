// SPDX-License-Identifier: BSD-3-Clause

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use tracing::{debug, error, info, warn};
use tracing_subscriber::{
    self, Layer,
    filter::{EnvFilter, LevelFilter},
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

type Error = Box<dyn std::error::Error>;
type Result = core::result::Result<(), Error>;

fn main() {
    tracing_subscriber::registry()
        .with(
            fmt::layer().with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .with_env_var("TAPERIPPER_XTASK_LOG_LEVEL")
                    .from_env_lossy(),
            ),
        )
        .init();

    info!("Hello, world!");
}

fn contrib_dir() -> PathBuf {
    project_root().join("contrib")
}

fn target_dir() -> PathBuf {
    project_root().join("target")
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
