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
        pi::mp::{CpuPhysicalLocation, MpServices, ProcessorInformation},
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

    Ok(if img_opts.len() > 0 {
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

pub fn get_cpu_info(cpu: usize) -> Result<ProcessorInformation, uefi::Error> {
    let mp = get_proto::<MpServices>()?;

    Ok(mp.get_processor_info(cpu)?)
}

pub fn get_core_count() -> Result<(usize, usize), uefi::Error> {
    let mp = get_proto::<MpServices>()?;
    let proc_count = mp.get_number_of_processors()?;

    Ok((proc_count.total, proc_count.enabled))
}

pub fn get_current_core() -> Result<usize, uefi::Error> {
    let mp = get_proto::<MpServices>()?;

    Ok(mp.who_am_i()?)
}

pub fn get_current_core_info() -> Result<ProcessorInformation, uefi::Error> {
    let mp = get_proto::<MpServices>()?;

    Ok(mp.get_processor_info(mp.who_am_i()?)?)
}
