# Taperipper

> [!WARNING]
> Taperipper uses quite a few experimental Rust features, as such it requires a nightly toolchain.
> It has been built on and tested with 1.88.0-nightly at the earliest.

Taperipper is a UEFI based Linux bootloader designed to boot modern Linux on modern hardware from 9-track reel-to-reel magnetic tape. It leverages the [Squishy] hardware platform to interface with SCSI based 9-track streamers to load the kernel and initramfs off of tape. As such, it's not generally useful except maybe as being gutted for other UEFI based projects.

Along with the main Taperipper loader there are two support crates, [`tinypng`] and [`iosevka-embedded`].

The [`tinypng`] crate is a standalone `no_std` PNG deserialize with transparency support for [embedded-graphics]. It's designed to be run on larger SoC's and bare metal systems with enough grunt, and as such may not work for smaller embedded devices.

The [`iosevka-embedded`] crate is a font package to allow [embedded-graphics] to draw with the lovely [Iosevka] font by [be5invis]

## License

The majority of the code in Taperipper is licensed under the [BSD-3-Clause], the full text of which can be found in the [`LICENSE.BSD`] file.

Some components have been adapted from [mycelium] by [Eliza Weisman], are appropriately annotated, and fall under the [MIT] license, the full text of which can be found in the [`LICENSE.MIT`] file.

The non-generated parts of the [`iosevka-embedded`] code are under the [BSD-3-Clause]. The generated data files and BDF fonts are under the [OFL-1.1], the full text of which can be found in the [`LICENSE.OFL`] file.

[Squishy]: https://github.com/squishy-scsi/squishy
[`tinypng`]: ./tinypng
[embedded-graphics]: https://github.com/embedded-graphics/embedded-graphics
[`iosevka-embedded`]: ./iosevka-embedded
[Iosevka]: https://github.com/be5invis/Iosevka
[be5invis]: https://typeof.net/
[mycelium]: https://github.com/hawkw/mycelium
[Eliza Weisman]: https://www.elizas.website/
[BSD-3-Clause]: https://spdx.org/licenses/BSD-3-Clause.html
[`LICENSE.BSD`]: ./LICENSE.BSD
[MIT]: https://spdx.org/licenses/MIT.html
[`LICENSE.MIT`]: ./LICENSE.MIT
[OFL-1.1]: https://spdx.org/licenses/OFL-1.1.html
[`LICENSE.OFL`]: ./LICENSE.OFL
