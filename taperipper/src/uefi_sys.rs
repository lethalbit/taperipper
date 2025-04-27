// SPDX-License-Identifier: BSD-3-Clause
// UEFI Helpers/Utilities

use std::os::uefi as uefi_std;
use tracing::info;
use uefi::{
    Handle, Status, boot, proto,
    runtime::{self, ResetType},
    table,
};

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

pub fn get_proto<P>() -> Result<boot::ScopedProtocol<P>, uefi::Error>
where
    P: proto::Protocol,
{
    boot::get_handle_for_protocol::<P>().and_then(|hndl| boot::open_protocol_exclusive::<P>(hndl))
}
