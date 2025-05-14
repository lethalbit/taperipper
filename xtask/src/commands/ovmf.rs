// SPDX-License-Identifier: BSD-3-Clause

use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    process,
};

use git2::Repository;
use tracing::{debug, info, warn};

const EDKII_REPO: &str = "https://github.com/tianocore/edk2.git";
const EDKII_TAG: &str = "edk2-stable202408.01";

pub fn build_firmware(args: &clap::ArgMatches) -> crate::utils::Result {
    // Some path constants for EDKII
    #[allow(non_snake_case)]
    let OVMF_BUILD = crate::paths::edk2_dir()
        .join("Build")
        .join("OvmfX64")
        .join("DEBUG_GCC");
    #[allow(non_snake_case)]
    let OVMF_BUILD_FV = OVMF_BUILD.join("FV");

    // Build arguments
    #[allow(non_snake_case)]
    let BUILD_ARGS = &[
        "-DFD_SIZE_4MB",
        "-DNETWORK_HTTP_BOOT_ENABLED",
        "-DNETWORK_IP6_ENABLE",
        "-DTPM_CONFIG_ENABLE",
        "-DTPM_ENABLE",
        "-DTPM1_ENABLE",
        "-DTPM2_ENABLE",
    ];

    // If we have an OVMF firmware file, we can bail early
    if crate::paths::ovmf_file_code().exists() {
        info!(
            "{:?} exists, don't need to build OVMF",
            crate::paths::ovmf_file_code()
        );
        return Ok(());
    }

    // Double check to make sure the `.ovmf` and `.ovmf/img` directories exist
    let _ = crate::utils::need_dir(crate::paths::ovmf_img_dir())?;

    let mut sh = process::Command::new(env::var("SHELL").unwrap_or("sh".to_string()));
    let mut make = process::Command::new(env::var("MAKE").unwrap_or("make".to_string()));

    if !crate::paths::edk2_dir().exists() {
        info!("Cloning EDK II Repo");
        let repo = Repository::clone(EDKII_REPO, crate::paths::edk2_dir())?;

        info!("Checking out tag {}", EDKII_TAG);
        let (obj, reference) = repo.revparse_ext(EDKII_TAG)?;
        repo.checkout_tree(&obj, None)?;
        match reference {
            Some(refer) => repo.set_head(refer.name().unwrap()),
            None => repo.set_head_detached(obj.id()),
        }?;

        info!("Initializing submodules");
        let submodules = repo.submodules()?;
        for mut sm in submodules {
            sm.update(true, None)?;
        }

        info!("Building base tools");
        if !make
            .current_dir(crate::paths::edk2_dir())
            .args(&["-C", "BaseTools"])
            .env("CC", "gcc-13")
            .status()?
            .success()
        {
            Err("Unable to build OVMF")?;
        }
    }

    info!("EDK II checked out...");

    if !OVMF_BUILD_FV.join("OVMF_CODE.fd").exists() {
        info!("OVMF_CODE not found, building...");

        if !sh
            .current_dir(crate::paths::edk2_dir())
            .args(&[
                "-c",
                format!(
                    "source edksetup.sh && BaseTools/BinWrappers/PosixLike/build -p OvmfPkg/OvmfPkgX64.dsc -a X64 -b DEBUG -t GCC {}",
                    BUILD_ARGS.join(" ").as_str()
                ).as_str()
            ]).status()?.success() {
                Err("Unable to build OVMF")?;
            }
    }

    info!("OVMF built...");

    info!(
        "Copying OVMF_CODE and OVMF_VARS to {:?} if needed",
        crate::paths::ovmf_dir()
    );
    crate::utils::copy_if_newer(
        OVMF_BUILD_FV.join("OVMF_CODE.fd"),
        crate::paths::ovmf_file_code(),
    )?;
    crate::utils::copy_if_newer(
        OVMF_BUILD_FV.join("OVMF_VARS.fd"),
        crate::paths::ovmf_file_vars(),
    )?;

    info!("Copying OVMF Debug symbols and binaries if needed");
    for img in crate::utils::filter_dir(OVMF_BUILD.join("X64"), "efi")? {
        let basename = img.file_name().unwrap();
        let tgt = crate::paths::ovmf_img_dir().join(basename);

        debug!("Copying {:?} to {:?}", &basename, &tgt);

        crate::utils::copy_if_ne(img, tgt)?;
    }

    for img in crate::utils::filter_dir(OVMF_BUILD.join("X64"), "debug")? {
        let basename = img.file_name().unwrap();
        let tgt = crate::paths::ovmf_img_dir().join(basename);

        debug!("Copying {:?} to {:?}", &basename, &tgt);

        crate::utils::copy_if_ne(img, tgt)?;
    }

    info!("OVMF setup completed");

    Ok(())
}

pub fn build_debug(args: &clap::ArgMatches) -> crate::utils::Result {
    #[allow(non_snake_case)]
    let OVMF_DBG_LOG = crate::paths::ovmf_dir().join("qemu.log");

    if crate::paths::ovmf_gdb_prelude().exists() {
        info!("OVMF GDB Debug script prelude exists, no need to regen");
        return Ok(());
    }

    if !crate::utils::need_dir(crate::paths::ovmf_esp())? {
        // Dump our startup script so we exit QEMU when we start
        let shell_startup = crate::paths::ovmf_esp().join("startup.nsh");
        fs::write(shell_startup, "reset -s")?;
    }

    if !OVMF_DBG_LOG.exists() {
        info!("OVMF Debug log does not exist, generating");

        if !crate::utils::common_run_qemu(&crate::paths::ovmf_esp())
            .current_dir(crate::paths::ovmf_dir())
            .args(&[
                "-enable-kvm",
                "-debugcon",
                format!("file:{}", OVMF_DBG_LOG.display()).as_str(),
                "-global",
                "isa-debugcon.iobase=0x402",
            ])
            .status()?
            .success()
        {
            Err("Unable to generate OVMF startup debug logs")?;
        }
    }

    info!("Getting loaded EFI modules");

    let mut gdb_prelude = BufWriter::new(File::create(crate::paths::ovmf_gdb_prelude())?);

    for line in BufReader::new(File::open(OVMF_DBG_LOG)?)
        .lines()
        .map(|line| line.ok())
        .filter(|line| line.is_some())
        .map(|line| line.unwrap())
        .filter(|line| line.starts_with("Loading"))
        .filter(|line| line.ends_with(".efi"))
    {
        let mut parts = line.rsplit(" ").take(2);
        let efi_img = parts.next().unwrap();
        let load_addr = crate::utils::from_hex(parts.next().unwrap().split("=").nth(1).unwrap())?;

        debug!("Found EFI Image {} loaded at {:#018x}", efi_img, load_addr);

        let img_path = crate::paths::ovmf_img_dir().join(efi_img);
        let mut dbg_path = img_path.clone();
        let _ = dbg_path.set_extension("debug");

        // TODO(aki): There are cases with CpuDxe and such where there are multiple based on GUID suffix
        if !img_path.exists() {
            warn!("OVMF Image {} wasn't found?", img_path.display());
            continue;
        }

        if !dbg_path.exists() {
            warn!(
                "debug info for OVMF Image {} wasn't found?",
                img_path.display()
            );
            continue;
        }

        let buff = fs::read(&img_path)?;
        let obj = goblin::pe::PE::parse(&buff);

        if let Err(obj_err) = obj {
            warn!("Skipping {}, unable to read PE file.", img_path.display());
            warn!("Parse Error: {obj_err}");
            continue;
        }

        let obj = obj.unwrap();

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

        gdb_prelude.write(
            format!(
                "add-symbol-file {} -s .text {:#018x} -s .date {:#018x}\n",
                dbg_path.display(),
                text_rebase,
                data_rebase
            )
            .as_bytes(),
        )?;
    }

    info!("Done writing OVMF GDB prelude");

    Ok(())
}
