// SPDX-License-Identifier: BSD-3-Clause
// UEFI Helpers/Utilities
#![allow(dead_code)]

use core::{ffi::c_void, ptr};
use std::os::uefi as uefi_std;
use tracing::{debug, info, warn};
use uefi::{
    CStr16, CString16, Handle, Status, boot,
    proto::{
        self,
        console::gop::{GraphicsOutput, PixelFormat},
        loaded_image::{LoadOptionsError, LoadedImage},
        media::{
            file::{File, FileAttribute, FileMode},
            fs::SimpleFileSystem,
        },
        misc::Timestamp,
    },
    runtime::{self, ResetType, VariableVendor},
    system,
    table::{
        self,
        cfg::{ACPI_GUID, ACPI2_GUID, SMBIOS_GUID, SMBIOS3_GUID},
    },
};
use uefi_raw::protocol::misc::TimestampProperties;

pub fn get_acpi_table() -> Option<(u8, *const c_void)> {
    system::with_config_table(|table| {
        let mut table_ptr = ptr::null();
        let mut table_ver: u8 = 0;

        for entry in table {
            match entry.guid {
                ACPI_GUID => {
                    if table_ver < 1 {
                        table_ver = 1;
                        table_ptr = entry.address;
                    }
                }
                ACPI2_GUID => {
                    if table_ver < 2 {
                        table_ver = 2;
                        table_ptr = entry.address;
                    }
                }
                _ => {}
            }
        }

        if table_ptr.is_null() {
            None
        } else {
            Some((table_ver, table_ptr))
        }
    })
}

pub fn get_smbios_table() -> Option<(u8, *const c_void)> {
    system::with_config_table(|table| {
        let mut table_ptr = ptr::null();
        let mut table_ver: u8 = 0;

        for entry in table {
            match entry.guid {
                SMBIOS_GUID => {
                    if table_ver < 1 {
                        table_ver = 1;
                        table_ptr = entry.address;
                    }
                }
                SMBIOS3_GUID => {
                    if table_ver < 3 {
                        table_ver = 3;
                        table_ptr = entry.address;
                    }
                }
                _ => {}
            }
        }
        if table_ptr.is_null() {
            None
        } else {
            Some((table_ver, table_ptr))
        }
    })
}

pub fn init_uefi() {
    // Get the system table and image handle
    let system_table = uefi_std::env::system_table();
    let image_handle = uefi_std::env::image_handle();

    // Setup the UEFI crate
    unsafe {
        table::set_system_table(system_table.as_ptr().cast());
        let image_handle = Handle::from_ptr(image_handle.as_ptr()).unwrap();
        boot::set_image_handle(image_handle);
    }
}

pub fn has_boot_services() -> bool {
    // Try to get the System table
    let Some(system_table) = table::system_table_raw() else {
        return false;
    };

    // Get a handle to the system table and check if the boot services are there
    let system_table = unsafe { system_table.as_ref() };
    !system_table.boot_services.is_null()
}

// Reboot the machine
pub fn reboot(status: Option<Status>, data: Option<&[u8]>) -> ! {
    info!("Rebooting system");

    runtime::reset(ResetType::COLD, status.unwrap_or(Status::SUCCESS), data);
}
pub fn reboot_now() -> ! {
    reboot(None, None);
}

// Shutdown the machine
pub fn shutdown(status: Option<Status>, data: Option<&[u8]>) -> ! {
    info!("Shutting system down");

    runtime::reset(ResetType::SHUTDOWN, status.unwrap_or(Status::SUCCESS), data);
}
pub fn shutdown_now() -> ! {
    shutdown(None, None);
}

// Set the highest rest output mod we can
pub fn set_best_stdout_mode() {
    system::with_stdout(|stdout| {
        let best = stdout.modes().last().unwrap();
        if stdout.set_mode(best).is_err() {
            warn!(
                "Unable to set output mode to {}x{}",
                best.columns(),
                best.rows()
            );
        } else {
            debug!("Set output mode to {}x{}", best.columns(), best.rows());
        }
    });
}

pub fn get_proto<P>() -> Result<boot::ScopedProtocol<P>, uefi::Error>
where
    P: proto::Protocol,
{
    boot::get_handle_for_protocol::<P>().and_then(|hndl| boot::open_protocol_exclusive::<P>(hndl))
}

pub fn get_options() -> Result<Option<Vec<String>>, uefi::Error> {
    let loaded = get_proto::<LoadedImage>()?;

    let img_opts = loaded.load_options_as_cstr16();
    if let Err(opts_err) = img_opts {
        return match opts_err {
            LoadOptionsError::NotSet => Ok(None),
            _ => Err(uefi::Error::new(uefi::Status::ABORTED, ())),
        };
    }

    let mut img_opts: Vec<String> = img_opts
        .unwrap()
        .to_string()
        .split(' ')
        .map(str::to_string)
        .collect();

    img_opts.shrink_to_fit();

    Ok(if !img_opts.is_empty() {
        Some(img_opts)
    } else {
        None
    })
}

pub fn get_image_info() -> Result<(usize, usize), uefi::Error> {
    let loaded = get_proto::<LoadedImage>()?;
    let img_info = loaded.info();

    Ok((img_info.0 as usize, img_info.1 as usize))
}

pub fn init_graphics(
    max_width: usize,
    max_height: usize,
) -> Result<boot::ScopedProtocol<GraphicsOutput>, uefi::Error> {
    let mut gop = get_proto::<GraphicsOutput>()?;

    // Pull out all the viable Video modes
    let mut viable_modes = gop
        .modes()
        .enumerate()
        .filter(|mode| {
            let mode_info = mode.1.info();
            let pixle_fmt = mode_info.pixel_format();

            if pixle_fmt == PixelFormat::Rgb || pixle_fmt == PixelFormat::Bgr {
                let (m_width, m_height) = mode_info.resolution();
                (m_width <= max_width) && (m_height <= max_height)
            } else {
                false
            }
        })
        .map(|mode| (mode.0, mode.1.info().resolution()))
        .collect::<Vec<(usize, (usize, usize))>>();

    // Sort them
    viable_modes.sort_by(|m1, m2| m1.1.partial_cmp(&m2.1).unwrap());

    // The last mode should be what we want
    let wanted_mode = viable_modes.last().unwrap().0;

    let new_mode = gop
        .modes()
        .nth(wanted_mode)
        .ok_or(uefi::Error::new(uefi::Status::INVALID_PARAMETER, ()))?;
    let _ = gop.set_mode(&new_mode);

    Ok(gop)
}

pub fn read_slice(file_path: &str, from: u64, buff: &mut [u8]) -> Result<usize, uefi::Error> {
    let mut fs = get_proto::<SimpleFileSystem>()?;

    // Convert the sane string to the absurd UCS-2 bullshit (thanks Microsoft /s)
    let usc2_path =
        CString16::try_from(file_path).map_err(|_| uefi::Error::new(uefi::Status::ABORTED, ()))?;

    // Open the volume, then open the file itself.
    let mut file = fs
        .open_volume()
        .and_then(|mut vol| {
            vol.open(
                CStr16::from_u16_with_nul(usc2_path.to_u16_slice_with_nul()).unwrap(),
                FileMode::ReadWrite,
                FileAttribute::empty(),
            )
        })?
        .into_regular_file()
        .ok_or(uefi::Error::new(uefi::Status::ABORTED, ()))?;

    file.set_position(from)?;

    // Fill the slice from the file
    file.read(buff)
        .map_err(|_| uefi::Error::new(uefi::Status::ABORTED, ()))
}

pub fn get_var(name: &str) -> Option<Box<[u8]>> {
    let mut buf: Vec<u16> = Vec::with_capacity(name.chars().count() * 4);

    let var_name = CStr16::from_str_with_buf(name, buf.as_mut_slice()).ok()?;

    if let Ok(var) = runtime::get_variable_boxed(var_name, &VariableVendor::GLOBAL_VARIABLE) {
        Some(var.0)
    } else {
        None
    }
}

pub fn get_timestamp_properties() -> Result<TimestampProperties, uefi::Error> {
    let ts = get_proto::<Timestamp>()?;
    ts.get_properties()
}

pub fn get_timestamp() -> u64 {
    let ts = get_proto::<Timestamp>().unwrap();
    ts.get_timestamp()
}
