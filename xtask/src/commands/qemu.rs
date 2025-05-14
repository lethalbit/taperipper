// SPDX-License-Identifier: BSD-3-Clause

use std::fs;

use tracing::debug;

pub fn run(args: &clap::ArgMatches) -> crate::utils::Result {
    // Ensure we're up to date
    let _ = crate::commands::build_all(&args)?;

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
