// SPDX-License-Identifier: BSD-3-Clause

pub mod check {
    use std::{env, process};

    use clap::{ArgMatches, Command};

    use crate::utils;

    pub const COMMAND_NAME: &str = "check";

    pub fn init() -> Command {
        crate::commands::cmd_common(Command::new(COMMAND_NAME).about("Run clippy on Taperipper"))
    }

    pub fn exec(args: &ArgMatches) -> utils::Result {
        let mut cargo = process::Command::new(env::var("CARGO").unwrap_or("cargo".to_string()));

        let tar_type: crate::utils::TargetType = args.into();

        if !cargo
            .current_dir(crate::paths::project_root())
            .args(&[
                "clippy",
                "--bin",
                "taperipper",
                // NOTE(aki): Because cargo can't just figure out to use this we have to specify it manually
                "--config",
                crate::paths::project_root()
                    .join("taperipper")
                    .join(".cargo")
                    .join("config.toml")
                    .to_str()
                    .unwrap(),
                "--profile",
                match tar_type {
                    crate::utils::TargetType::Release => "release",
                    crate::utils::TargetType::Debug => "dev",
                },
            ])
            .status()?
            .success()
        {
            Err("Unable to build taperipper")?;
        }

        Ok(())
    }
}

pub mod build {
    use std::{
        env,
        fs::{self, File},
        io::{BufWriter, Write},
        process,
    };

    use clap::{ArgMatches, Command};
    use tracing::{debug, info};

    use crate::utils;

    pub const COMMAND_NAME: &str = "build-taperipper";

    pub fn init() -> Command {
        crate::commands::cmd_common(
            Command::new(COMMAND_NAME).about("Build the Taperipper UEFI image"),
        )
    }

    pub fn exec(args: &ArgMatches) -> utils::Result {
        let mut cargo = process::Command::new(env::var("CARGO").unwrap_or("cargo".to_string()));

        let tar_type: crate::utils::TargetType = args.into();

        info!("Building taperipper UEFI image");

        if !cargo
            .current_dir(crate::paths::project_root())
            .args(&[
                "build",
                "--bin",
                "taperipper",
                // NOTE(aki): Because cargo can't just figure out to use this we have to specify it manually
                "--config",
                crate::paths::project_root()
                    .join("taperipper")
                    .join(".cargo")
                    .join("config.toml")
                    .to_str()
                    .unwrap(),
                "--profile",
                match tar_type {
                    crate::utils::TargetType::Release => "release",
                    crate::utils::TargetType::Debug => "dev",
                },
            ])
            .status()?
            .success()
        {
            Err("Unable to build taperipper")?;
        }

        info!("Done...");

        let efi_img = crate::paths::target_dir_for_type(tar_type).join("taperipper.efi");

        let buff = fs::read(&efi_img)?;
        let obj = goblin::pe::PE::parse(&buff)?;

        let text = (obj.sections)
            .iter()
            .filter(|s| s.name().unwrap() == ".text")
            .nth(0)
            .ok_or::<crate::utils::Error>("No .text section!".into())?;

        let data = (obj.sections)
            .iter()
            .filter(|s| s.name().unwrap() == ".data")
            .nth(0)
            .ok_or::<crate::utils::Error>("No .data section!".into())?;

        let rdata = (obj.sections)
            .iter()
            .filter(|s| s.name().unwrap() == ".rdata")
            .nth(0)
            .ok_or::<crate::utils::Error>("No .rdata section!".into())?;

        // HACK(aki): OVMF seems to *always* load us here, so we just kinda bet on it for debug
        let load_addr: u64 = 0x0003DD72000;

        let text_rebase = text.virtual_address as u64 + load_addr;
        debug!(
            "Rebased .text load addr from {:#018x} to {:#018x}",
            text.virtual_address, text_rebase
        );
        let data_rebase = data.virtual_address as u64 + load_addr;
        debug!(
            "Rebased .date load addr from {:#018x} to {:#018x}",
            data.virtual_address, data_rebase
        );
        let rdata_rebase = rdata.virtual_address as u64 + load_addr;
        debug!(
            "Rebased .rdate load addr from {:#018x} to {:#018x}",
            rdata.virtual_address, data_rebase
        );

        let mut gdb_script =
            BufWriter::new(File::create(crate::paths::target_dir().join(".gdbinit"))?);

        gdb_script
            .write(format!("source {}\n", crate::paths::ovmf_gdb_prelude().display()).as_bytes())?;
        gdb_script.write(
            format!(
                "add-symbol-file {} -s .text {:#018x} -s .data {:#018x} -s .rdata {:#018x}\n",
                efi_img.display(),
                text_rebase,
                data_rebase,
                rdata_rebase
            )
            .as_bytes(),
        )?;

        // NOTE(aki): OVMF always loads us here, and debugging is painful without symbols.
        gdb_script.write(
            format!(
                "add-symbol-file {} -s .text 0x000000003fe36000\n",
                efi_img.display()
            )
            .as_bytes(),
        )?;

        gdb_script.write("tar remote 127.0.0.1:1234\n".as_bytes())?;

        Ok(())
    }
}
