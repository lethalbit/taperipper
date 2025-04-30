// SPDX-License-Identifier: BSD-3-Clause
// UEFI Helpers/Utilities

use core::{ffi::c_void, ptr};
use std::os::uefi as uefi_std;
use tracing::{debug, info, warn};
use uefi::{
    Handle, Status, boot,
    proto::{
        self,
        console::gop::{GraphicsOutput, PixelFormat},
    },
    runtime::{self, ResetType},
    system,
    table::{
        self,
        cfg::{ACPI_GUID, ACPI2_GUID, ConfigTableEntry, SMBIOS_GUID, SMBIOS3_GUID},
    },
};

#[derive(Clone, Copy, Debug)]
pub struct ExtraTables {
    pub acpi: *const c_void,
    pub acpi_ver: u8,
    pub smbios: *const c_void,
    pub smbios_ver: u8,
}

impl ExtraTables {
    pub fn new(cfg_table: &[ConfigTableEntry]) -> Self {
        let mut cfg = Self {
            acpi: ptr::null(),
            acpi_ver: 0,
            smbios: ptr::null(),
            smbios_ver: 0,
        };
        cfg.populate(cfg_table);
        cfg
    }

    fn populate(&mut self, cfg_table: &[ConfigTableEntry]) {
        for table_entry in cfg_table {
            match table_entry.guid {
                ACPI_GUID => {
                    if self.acpi_ver < 1 {
                        self.acpi_ver = 1;
                        self.acpi = table_entry.address;
                    }
                }
                ACPI2_GUID => {
                    if self.acpi_ver < 2 {
                        self.acpi_ver = 2;
                        self.acpi = table_entry.address;
                    }
                }
                SMBIOS_GUID => {
                    if self.smbios_ver < 1 {
                        self.smbios_ver = 1;
                        self.smbios = table_entry.address;
                    }
                }
                SMBIOS3_GUID => {
                    if self.smbios_ver < 3 {
                        self.smbios_ver = 3;
                        self.smbios = table_entry.address;
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn init_uefi() {
    // Get the system table and image handle
    let system_table = uefi_std::env::system_table();
    let image_handle = uefi_std::env::image_handle();

    // Setup the UEFI crate
    unsafe {
        table::set_system_table(system_table.as_ptr().cast());
        let image_handle = Handle::from_ptr(image_handle.as_ptr().cast()).unwrap();
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
    return !system_table.boot_services.is_null();
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
        if let Err(_) = stdout.set_mode(best) {
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
