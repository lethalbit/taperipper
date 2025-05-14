// SPDX-License-Identifier: BSD-3-Clause

use crate::utils;

mod ovmf;
mod qemu;
mod taperipper;

pub fn setup_commands(command: clap::Command) -> clap::Command {
    let arg_target_type = clap::Arg::new("TARGET_TYPE")
        .value_parser(["debug", "release"])
        .default_value("debug")
        .help("h");

    command
        .subcommand(
            clap::Command::new("run-qemu")
                .about("Run the Taperipper UEFI image in QEMU")
                .arg(arg_target_type.clone()),
        )
        .subcommand(
            clap::Command::new("build")
                .about("Build everything if needed")
                .arg(arg_target_type.clone()),
        )
        .subcommand(
            clap::Command::new("build-taperipper")
                .about("Build only the taperipper UEFI image if needed")
                .arg(arg_target_type.clone()),
        )
        .subcommand(
            clap::Command::new("build-ovmf-fw").about("Build only the OVMF firmware if needed"),
        )
        .subcommand(
            clap::Command::new("build-ovmf-dbg")
                .about("Build only the OVMF debug symbols if needed"),
        )
}

pub fn dispatch(args: clap::ArgMatches) -> utils::Result {
    match args.subcommand().unwrap() {
        ("run-qemu", args) => qemu::run(args),
        ("build", args) => build_all(args),
        ("build-taperipper", args) => taperipper::build(args),
        ("build-ovmf-fw", args) => ovmf::build_firmware(args),
        ("build-ovmf-dbg", args) => ovmf::build_debug(args),
        (_, __) => unreachable!(),
    }
}

fn build_all(args: &clap::ArgMatches) -> utils::Result {
    // Build the OVMF if we don't have it
    let _ = ovmf::build_firmware(args)?;
    // Built the OVMF debug info setup next if we need to
    let _ = ovmf::build_debug(args)?;
    // Finally build the taperipper UEFI image itself
    taperipper::build(args)
}
