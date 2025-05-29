// SPDX-License-Identifier: BSD-3-Clause

use uefi::proto::misc::Timestamp;
use uefi_raw::protocol::misc::TimestampProperties;

use crate::platform::uefi::get_proto;

pub fn get_timestamp_properties() -> Result<TimestampProperties, uefi::Error> {
    let ts = get_proto::<Timestamp>()?;
    ts.get_properties()
}

pub fn get_timestamp() -> u64 {
    let ts = get_proto::<Timestamp>().unwrap();
    ts.get_timestamp()
}
