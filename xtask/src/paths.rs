// SPDX-License-Identifier: BSD-3-Clause
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::utils::TargetType;

pub fn ovmf_gdb_prelude() -> PathBuf {
    ovmf_dir().join("prelude.gdb")
}

pub fn ovmf_file_code() -> PathBuf {
    ovmf_dir().join("OVMF_CODE.4m.fd")
}

pub fn ovmf_file_vars() -> PathBuf {
    ovmf_dir().join("OVMF_VARS.4m.fd")
}

pub fn ovmf_esp() -> PathBuf {
    ovmf_dir().join("esp")
}

pub fn ovmf_img_dir() -> PathBuf {
    ovmf_dir().join("efi")
}

pub fn edk2_dir() -> PathBuf {
    target_dir().join(".edk2.git")
}

pub fn ovmf_dir() -> PathBuf {
    target_dir().join(".ovmf")
}

pub fn efi_boot_dir() -> PathBuf {
    efi_root().join("EFI").join("boot")
}

pub fn efi_root() -> PathBuf {
    target_dir().join("esp")
}

pub fn contrib_dir() -> PathBuf {
    project_root().join("contrib")
}

pub fn target_dir_for_type(tar_type: TargetType) -> PathBuf {
    match tar_type {
        TargetType::Release => target_release(),
        TargetType::Debug => target_debug(),
    }
}

pub fn target_debug() -> PathBuf {
    target_dir().join("x86_64-unknown-uefi").join("debug")
}

pub fn target_release() -> PathBuf {
    target_dir().join("x86_64-unknown-uefi").join("release")
}

pub fn uefi_vars() -> PathBuf {
    target_dir().join("uefi-vars.json")
}

pub fn target_dir() -> PathBuf {
    project_root().join("target")
}

pub fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
