// SPDX-License-Identifier: BSD-3-Clause

use uefi::{
    CStr16, Guid, guid,
    runtime::{self, VariableAttributes, VariableVendor},
};

pub const TAPERIPPER_UEFI_NAMESPACE: Guid = guid!("70a40a42-5ee6-4620-ad7c-97567d038a20");
pub const TAPERIPPER_UEFI_VENDOR: VariableVendor = VariableVendor(TAPERIPPER_UEFI_NAMESPACE);

pub fn get(name: &str) -> Option<Box<[u8]>> {
    let mut enc = name.encode_utf16().collect::<Vec<_>>();
    enc.push(0x00);

    let var_name = CStr16::from_u16_until_nul(enc.as_slice()).ok()?;

    if let Ok(var) = runtime::get_variable_boxed(var_name, &TAPERIPPER_UEFI_VENDOR) {
        Some(var.0)
    } else {
        None
    }
}

pub fn set(name: &str, data: &[u8]) {
    let mut enc = name.encode_utf16().collect::<Vec<_>>();
    enc.push(0x00);

    let var_name = CStr16::from_u16_until_nul(enc.as_slice()).unwrap();

    runtime::set_variable(
        var_name,
        &TAPERIPPER_UEFI_VENDOR,
        VariableAttributes::BOOTSERVICE_ACCESS
            | VariableAttributes::RUNTIME_ACCESS
            | VariableAttributes::NON_VOLATILE,
        data,
    )
    .unwrap();
}
