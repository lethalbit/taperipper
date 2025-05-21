// SPDX-License-Identifier: BSD-3-Clause

use clap::{Arg, ArgMatches, Command};

use crate::utils;

mod ovmf;
mod qemu;
mod taperipper;

pub type CmdExec = fn(&ArgMatches) -> utils::Result;

// The "meta" build command
pub mod build {
    use clap::{ArgMatches, Command};

    use crate::utils;

    use super::{ovmf, taperipper};

    pub const COMMAND_NAME: &str = "build";

    pub fn init() -> Command {
        crate::commands::cmd_common(Command::new(COMMAND_NAME))
    }

    pub fn exec(args: &ArgMatches) -> utils::Result {
        crate::commands::exec(ovmf::debug::COMMAND_NAME).ok_or("")?(args)?;
        crate::commands::exec(taperipper::build::COMMAND_NAME).ok_or("")?(args)?;

        Ok(())
    }
}

pub fn cmd_common(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("TARGET_TYPE")
            .value_parser(["debug", "release"])
            .default_value("debug"),
    )
}

pub fn init() -> Vec<Command> {
    vec![
        build::init(),
        ovmf::firmware::init(),
        ovmf::debug::init(),
        qemu::run::init(),
        qemu::shell::init(),
        taperipper::build::init(),
    ]
}

pub fn exec(command: &str) -> Option<CmdExec> {
    match command {
        build::COMMAND_NAME => Some(build::exec),
        ovmf::firmware::COMMAND_NAME => Some(ovmf::firmware::exec),
        ovmf::debug::COMMAND_NAME => Some(ovmf::debug::exec),
        qemu::run::COMMAND_NAME => Some(qemu::run::exec),
        qemu::shell::COMMAND_NAME => Some(qemu::shell::exec),
        taperipper::build::COMMAND_NAME => Some(taperipper::build::exec),
        _ => None,
    }
}
