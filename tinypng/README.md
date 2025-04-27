# `tinypng`

> [!WARNING]
> `tinypng` is ***not*** production grade, it's a toy and may randomly break, explode, implode, or
> fail in any number of unpredictable and inconceivable ways. Use at your own risk.

`tinypng` is a `no_std` PNG decoder for use with [embedded-graphics] and inspired by [`tinytga`] and [`tinybmp`].

The primary reason for this crate is because I needed something with easy transparency support, and I enjoy working with PNGs much more than TARGA and bitmap files.

The target use case for this is also running on bare metal x86_64, not a small SoC or microcontroller, so it's likely more heavy memory and computation wise than would be reasonable on a highly resource constrained system.

## License

`tinypng` is licensed under the [BSD-3-Clause], the full text of which can be found in the [`LICENSE.BSD`] file in the root of this repository.

[`tinytga`]: https://github.com/embedded-graphics/tinytga
[`tinybmp`]: https://github.com/embedded-graphics/tinybmp
[embedded-graphics]: https://github.com/embedded-graphics/embedded-graphics
[BSD-3-Clause]: https://spdx.org/licenses/BSD-3-Clause.html
[`LICENSE.BSD`]: ../LICENSE.BSD
