// SPDX-License-Identifier: BSD-3-Clause

pub mod run {
    use std::fs;

    use clap::{ArgMatches, Command};
    use tracing::debug;

    use crate::utils;

    pub const COMMAND_NAME: &str = "run-qemu";

    pub fn init() -> Command {
        crate::commands::cmd_common(Command::new(COMMAND_NAME).about("Run Taperipper in QEMU"))
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

        let boot_img = crate::paths::efi_boot_dir().join("BOOTx64.efi");
        crate::utils::copy_if_newer(
            crate::paths::target_dir_for_type(tar_type).join("taperipper.efi"),
            boot_img,
        )?;

        if !crate::utils::common_run_qemu(&crate::paths::efi_root())
            .current_dir(crate::paths::ovmf_dir())
            .args(&["-enable-kvm", "-debugcon", "stdio"])
            .status()?
            .success()
        {
            Err("QEMU Exited with an error condition!")?;
        }

        Ok(())
    }
}
