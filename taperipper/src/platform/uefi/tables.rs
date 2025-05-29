// SPDX-License-Identifier: BSD-3-Clause

use core::{ffi::c_void, ptr};

use uefi::{
    system,
    table::cfg::{ACPI_GUID, ACPI2_GUID, SMBIOS_GUID, SMBIOS3_GUID},
};

pub fn get_acpi() -> Option<(u8, *const c_void)> {
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

pub fn get_smbios() -> Option<(u8, *const c_void)> {
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
