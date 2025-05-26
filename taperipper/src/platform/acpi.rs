// SPDX-License-Identifier: BSD-3-Clause

use std::{cell::OnceCell, ptr::NonNull};

use acpi::AcpiTables;
use maitake_sync::{Mutex, spin::InitOnce};
use tracing::{debug, trace, warn};

use crate::platform;

#[derive(Clone, Debug)]
pub struct Handler {}

// TODO(aki): We need to write our own allocator eventually:tm: but for now just use identity mapping
impl acpi::AcpiHandler for Handler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        unsafe {
            acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new(physical_address as *mut T).unwrap(),
                size,
                size,
                self.clone(),
            )
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {}
}

pub static ACPI_TABLES: InitOnce<Mutex<AcpiTables<Handler>>> = InitOnce::uninitialized();

pub fn init_tables() {
    let handler = Handler {};

    if let Some((version, address)) = platform::uefi::get_acpi_table() {
        debug!("ACPI v{} Address: {:#018x}", version, address as usize);

        let tbl = unsafe { acpi::AcpiTables::from_rsdp(handler, address as usize).unwrap() };

        if let Some(proc_info) = tbl.platform_info().ok().and_then(|pi| pi.processor_info) {
            let boot_proc = proc_info.boot_processor;
            let ap_procs = proc_info.application_processors;
            let ap_count = ap_procs.len();
            trace!(
                "Boot processor: id={} state={:?}",
                boot_proc.processor_uid, boot_proc.state
            );

            trace!("We have {ap_count} application processors:");
            if ap_count > platform::smp::MAX_CORES - 1 {
                warn!(
                    "Total number of processors exceeds supported core count! ({} > {})",
                    ap_count + 1, // Count the boot core
                    platform::smp::MAX_CORES
                );
            }

            for ap in ap_procs.iter() {
                trace!(" * id={:04} state={:?}", ap.processor_uid, ap.state);
            }
        }

        ACPI_TABLES.init(Mutex::new(tbl));
    } else {
        warn!("Was unable to initialize ACPI Tables!");
    }
}
