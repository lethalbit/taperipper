// SPDX-License-Identifier: BSD-3-Clause

use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
    process, u64,
};

use tracing::debug;

pub type Error = Box<dyn std::error::Error>;
pub type Result = core::result::Result<(), Error>;

#[derive(Debug, Clone, Copy)]
pub enum TargetType {
    Release,
    Debug,
}

impl From<&clap::ArgMatches> for TargetType {
    fn from(value: &clap::ArgMatches) -> Self {
        match value.get_one::<String>("TARGET_TYPE").unwrap().as_str() {
            "release" => TargetType::Release,
            _ => TargetType::Debug,
        }
    }
}

pub fn from_hex(hex: &str) -> core::result::Result<u64, Error> {
    let dig = hex
        .strip_prefix("0x")
        .unwrap_or(hex.strip_prefix("0X").unwrap_or(hex));

    Ok(u64::from_str_radix(dig, 16)?)
}

pub fn filter_dir<P>(
    path: P,
    suffix: &str,
) -> core::result::Result<impl Iterator<Item = PathBuf>, Error>
where
    P: AsRef<Path> + fmt::Debug,
{
    debug!("Iterating dir {:?} and filtering for {}", &path, &suffix);

    Ok(fs::read_dir(&path)?
        .map(|res| res.map(|ent| ent.path()))
        .filter_map(|ent| ent.ok())
        .filter(|path| path.extension().is_some())
        .filter(move |path| path.extension().unwrap() == suffix))
}

pub fn need_dir<P>(path: P) -> core::result::Result<bool, Error>
where
    P: AsRef<Path> + fmt::Debug,
{
    if !fs::exists(&path)? {
        debug!("Path {:?} does not exist, creating...", &path);
        fs::create_dir_all(&path)?;
        return Ok(false);
    }

    Ok(true)
}

pub fn is_newer<P, Q>(from: P, to: Q) -> core::result::Result<bool, Error>
where
    P: AsRef<Path> + fmt::Debug,
    Q: AsRef<Path> + fmt::Debug,
{
    if !fs::exists(&from)? {
        Err("Source file does not exist!")?;
    }

    // If the target file doesn't exist, it is by definition, older :v
    if !fs::exists(&to)? {
        debug!("{:?} does not exist, source is newer", &to);
        return Ok(true);
    }

    let from_mod = fs::metadata(&from)?.modified()?;
    let to_mod = fs::metadata(&to)?.modified()?;
    debug!("{:?} age {:?}", &from, from_mod);
    debug!("{:?} age {:?}", &to, to_mod);

    Ok(from_mod > to_mod)
}

pub fn copy_if_newer<P, Q>(from: P, to: Q) -> Result
where
    P: AsRef<Path> + fmt::Debug,
    Q: AsRef<Path> + fmt::Debug,
{
    if is_newer(&from, &to)? {
        debug!("Copying {:?} to {:?}", &from, &to);
        fs::copy(&from, &to)?;
    }

    Ok(())
}

pub fn copy_if_ne<P, Q>(from: P, to: Q) -> Result
where
    P: AsRef<Path> + fmt::Debug,
    Q: AsRef<Path> + fmt::Debug,
{
    if !fs::exists(&to)? {
        debug!("Copying {:?} to {:?}", &from, &to);
        fs::copy(&from, &to)?;
    }

    Ok(())
}

pub fn common_run_qemu(efi_root: &PathBuf) -> process::Command {
    let mut qemu =
        process::Command::new(env::var("QEMU").unwrap_or("qemu-system-x86_64".to_string()));

    qemu.args(&[
        "-drive",
        format!(
            "if=pflash,format=raw,readonly=on,file={}",
            crate::paths::ovmf_file_code().display()
        )
        .as_str(),
        "-drive",
        format!(
            "if=pflash,format=raw,readonly=on,file={}",
            crate::paths::ovmf_file_vars().display()
        )
        .as_str(),
        "-drive",
        format!("format=raw,file=fat:rw:{}", &efi_root.display()).as_str(),
        "-device",
        format!(
            "uefi-vars-x64,jsonfile={}",
            crate::paths::uefi_vars().display()
        )
        .as_str(),
    ]);

    qemu
}
