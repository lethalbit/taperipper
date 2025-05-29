// SPDX-License-Identifier: BSD-3-Clause

use tracing::info;
use uefi::{
    Status,
    runtime::{self, ResetType},
};

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
