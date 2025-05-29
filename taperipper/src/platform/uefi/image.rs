// SPDX-License-Identifier: BSD-3-Clause

use uefi::proto::loaded_image::LoadedImage;

use crate::platform::uefi::get_proto;

pub fn get_info() -> Result<(usize, usize), uefi::Error> {
    let loaded = get_proto::<LoadedImage>()?;
    let img_info = loaded.info();

    Ok((img_info.0 as usize, img_info.1 as usize))
}
