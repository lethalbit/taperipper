// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct UefiVar {
    pub name: String,
    pub guid: Uuid,
    pub attr: u32,
    pub data: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct UefiVars {
    pub version: u32,
    pub variables: Vec<UefiVar>,
}

pub mod shell {
    use clap::{ArgMatches, Command};

    use crate::utils;

    pub const COMMAND_NAME: &str = "uefi-shell";

    pub fn init() -> Command {
        Command::new(COMMAND_NAME)
    }

    pub fn exec(_args: &ArgMatches) -> utils::Result {
        if !crate::utils::common_run_qemu(None)
            .current_dir(crate::paths::ovmf_dir())
            .status()?
            .success()
        {
            Err("QEMU Exited with an error condition!")?;
        }

        Ok(())
    }
}

pub mod run {
    use std::{
        fs::{self, File},
        io::{BufReader, BufWriter, Write},
    };

    use clap::{Arg, ArgAction, ArgMatches, Command};
    use tracing::debug;

    use crate::{commands::qemu::UefiVars, utils};

    use super::UefiVar;

    pub const COMMAND_NAME: &str = "run-qemu";

    pub fn init() -> Command {
        crate::commands::cmd_common(
            Command::new(COMMAND_NAME)
                .about("Run Taperipper in QEMU")
                .arg(
                    Arg::new("CORES")
                        .short('c')
                        .long("cores")
                        .action(ArgAction::Set)
                        .value_name("CORES")
                        .default_value("4")
                        .value_parser(clap::value_parser!(usize))
                        .help("Number of CPU cores to use"),
                )
                .arg(
                    Arg::new("DEBUG")
                        .short('d')
                        .long("debug")
                        .action(ArgAction::SetTrue)
                        .help(""),
                ),
        )
    }

    pub fn exec(args: &ArgMatches) -> utils::Result {
        // Ensure we're up to date
        let _ = crate::commands::exec(crate::commands::build::COMMAND_NAME)
            .ok_or("Unable to get build exec")?(&args)?;

        let tar_type: crate::utils::TargetType = args.into();

        if !crate::paths::efi_boot_dir().exists() {
            debug!("EFI boot directory does not exist, creating");
            fs::create_dir_all(crate::paths::efi_boot_dir())?;
        }

        let cfg = if !crate::paths::uefi_vars().exists() {
            debug!("UEFI Variables don't exist, creating default");
            UefiVars::default()
        } else {
            debug!("Reading UEFI Variables");
            let efi_vafs = BufReader::new(File::open(crate::paths::uefi_vars())?);
            let cfg: UefiVars = serde_json::from_reader(efi_vafs)?;
            cfg
        };

        let boot_img = crate::paths::efi_boot_dir().join("BOOTx64.efi");
        crate::utils::copy_if_newer(
            crate::paths::target_dir_for_type(tar_type).join("taperipper.efi"),
            boot_img,
        )?;

        // TODO(aki): Debug logging setting
        // cfg.variables.push(UefiVar {
        //     name: "TAPERIPPER_LOG_LEVEL".to_string(),
        //     guid: TAPERIPPER_UUID.clone(),
        //     attr: 0x07, // TODO(aki): NON_VOLATILE (0x01) | BOOTSERVICE_ACCESS (0x02) | RUNTIME_ACCESS (0x04)
        //     data: "Debug"
        //         .as_bytes()
        //         .iter()
        //         .map(|b| format!("{:02X}", b))
        //         .collect::<Vec<_>>()
        //         .join(""),
        // });

        let mut efi_vars = BufWriter::new(File::create(crate::paths::uefi_vars())?);
        efi_vars.write(serde_json::to_string(&cfg)?.as_bytes())?;
        drop(efi_vars);

        let mut qemu = crate::utils::common_run_qemu(Some(&crate::paths::efi_root()));

        qemu.current_dir(crate::paths::ovmf_dir()).args(&[
            "-enable-kvm",
            "-debugcon",
            "stdio",
            "-smp",
            args.get_one::<usize>("CORES")
                .unwrap_or(&2)
                .to_string()
                .as_str(),
        ]);

        if args.get_flag("DEBUG") {
            qemu.args(&["-S", "-s"]);
        }

        if !qemu.status()?.success() {
            Err("QEMU Exited with an error condition!")?;
        }

        Ok(())
    }
}
