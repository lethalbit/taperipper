// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct UefiVar {
    name: String,
    guid: Uuid,
    attr: u32,
    data: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct UefiVars {
    version: u32,
    variables: Vec<UefiVar>,
}

pub mod run {
    use std::{
        fs::{self, File},
        io::{BufWriter, Write},
    };

    use clap::{Arg, ArgAction, ArgMatches, Command};
    use tracing::debug;

    use crate::{commands::qemu::UefiVars, utils};

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

        if !crate::paths::uefi_vars().exists() {
            debug!("UEFI Variables don't exist, creating");
            let mut efi_vars = BufWriter::new(File::create(crate::paths::uefi_vars())?);
            efi_vars.write(serde_json::to_string(&UefiVars::default())?.as_bytes())?;
        }

        let boot_img = crate::paths::efi_boot_dir().join("BOOTx64.efi");
        crate::utils::copy_if_newer(
            crate::paths::target_dir_for_type(tar_type).join("taperipper.efi"),
            boot_img,
        )?;

        if !crate::utils::common_run_qemu(&crate::paths::efi_root())
            .current_dir(crate::paths::ovmf_dir())
            .args(&[
                "-enable-kvm",
                "-debugcon",
                "stdio",
                "-smp",
                args.get_one::<usize>("CORES")
                    .unwrap_or(&2)
                    .to_string()
                    .as_str(),
            ])
            .status()?
            .success()
        {
            Err("QEMU Exited with an error condition!")?;
        }

        Ok(())
    }
}
