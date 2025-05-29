// SPDX-License-Identifier: BSD-3-Clause

use tracing::{debug, warn};
use uefi::{
    boot,
    proto::console::gop::{GraphicsOutput, PixelFormat},
    system,
};

use crate::platform::uefi::get_proto;

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
