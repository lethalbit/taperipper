// SPDX-License-Identifier: BSD-3-Clause

use std::os::uefi as uefi_std;
use uefi::{Handle, boot, proto, table};

pub mod image;
pub mod output;
pub mod system;
pub mod tables;
pub mod time;
pub mod variables;

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

pub fn get_proto<P>() -> Result<boot::ScopedProtocol<P>, uefi::Error>
where
    P: proto::Protocol,
{
    boot::get_handle_for_protocol::<P>().and_then(|hndl| boot::open_protocol_exclusive::<P>(hndl))
}
