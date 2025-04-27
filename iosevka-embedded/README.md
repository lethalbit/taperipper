# `iosevka-embedded`

`iosevka-embdded` is a rendering of the [Iosevka] font by [be5invis] into a format that [embedded-graphics] can use.

It leverages BDF files [generated] by [FontForge] from the source [Iosevka] font and serialized using the [eg-bdf] tools and library.

It is currently only the sizes `8`, `16`, `24`, and `32` of Iosevka Fixed-Extended with Normal, Thin, and Bold variants, for a total of 12 variations.

It only covers the ASCII range of glyphs.

## License

The non auto-generated code is licensed under the [BSD-3-Clause], the full text of which can be found in the [`LICENSE.BSD`] file in the root of this repository.

The auto-generated code, data tables, and BDF font files are licensed under the [OFL-1.1], the full text of which can be found in the [`LICENSE.OFL`] file in the root of this repository.

[Iosevka]: https://github.com/be5invis/Iosevka
[be5invis]: https://typeof.net/
[embedded-graphics]: https://github.com/embedded-graphics/embedded-graphics
[generated]: ../scripts/mkfonts.py
[FontForge]: https://fontforge.org/
[eg-bdf]: https://github.com/embedded-graphics/bdf
[BSD-3-Clause]: https://spdx.org/licenses/BSD-3-Clause.html
[`LICENSE.BSD`]: ../LICENSE.BSD
[OFL-1.1]: https://spdx.org/licenses/OFL-1.1.html
[`LICENSE.OFL`]: ../LICENSE.OFL
