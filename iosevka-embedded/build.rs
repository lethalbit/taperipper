// SPDX-License-Identifier: BSD-3-Clause

use eg_font_converter::{FontConverter, Mapping};
use std::env;
use std::fmt::Write;
use std::fs::{self, write};
use std::path::{Path, PathBuf};

const FONTS: &[&str] = &["IosevkaFixed-Extended"];
const FONT_SIZES: &[u8] = &[8, 16, 24, 32];
const FONT_STYLES: &[&str] = &["", "Italic"];
const FONT_WEIGHTS: &[&str] = &["", "Thin", "Bold", "Light"];

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let font_bdf_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts"));
    let output_dir = PathBuf::from(&std::env::var_os("OUT_DIR").expect("no OUT_DIR"));

    let font_gen_dir = output_dir.join("generated");
    if !fs::exists(&font_gen_dir).expect("Unable to resolve path") {
        fs::create_dir(&font_gen_dir).expect("Unable to make generated font directory");
    }

    let mut font_rs = String::new();

    for font in FONTS {
        for font_weight in FONT_WEIGHTS {
            for font_style in FONT_STYLES {
                for font_size in FONT_SIZES {
                    let bdf_font_name = format!("{font}{font_weight}{font_style}-{font_size}");
                    let bdf_font = font_bdf_dir
                        .join(bdf_font_name.clone())
                        .with_extension("bdf");

                    if !fs::exists(&bdf_font).unwrap() {
                        continue;
                    }

                    eprintln!("Converting {}...", bdf_font.to_str().unwrap());
                    println!("cargo::rerun-if-changed={}", bdf_font.to_str().unwrap());

                    let font_name = bdf_font_name.as_str().replace("-", "_").to_lowercase();

                    let outfile_rs = font_gen_dir.join(font_name.clone()).with_extension("rs");
                    let outfile_data = font_gen_dir.join(font_name.clone()).with_extension("data");

                    if !fs::exists(&outfile_rs).unwrap() || !fs::exists(&outfile_data).unwrap() {
                        let output = FontConverter::new(&bdf_font, &font_name.to_uppercase())
                            .missing_glyph_substitute('*')
                            // TODO(aki): Deal with more glyphs
                            .glyphs(Mapping::Ascii)
                            .glyphs('█'..='▏')
                            .glyphs(&['‼', '‽', '⁇', '⁈', '⁉', '№', '⍉', '➜'][..])
                            .glyphs('╭'..='╰')
                            .glyphs(&['─', '│', '├', '┤', '┬', '┴'][..])
                            .convert_eg_bdf()
                            .expect("Unable to convert BDF font");

                        if !fs::exists(&outfile_rs).unwrap() {
                            eprintln!("Writing {}", outfile_rs.to_str().unwrap());
                            write(outfile_rs, output.rust())
                                .expect("Unable to write output rust file");
                        }

                        if !fs::exists(&outfile_data).unwrap() {
                            eprintln!("Writing {}", outfile_data.to_str().unwrap());
                            write(outfile_data, output.data())
                                .expect("Unable to write output data file");
                        }
                    }

                    let feat_weight = format!("feature = \"{}\", ", font_weight.to_lowercase());
                    let feat_style = format!("feature = \"{}\", ", font_style.to_lowercase());
                    let feat_size = format!("feature = \"size_{}\"", font_size);

                    writeln!(
                        &mut font_rs,
                        r#"#[cfg(all({}{}{}))]"#,
                        if font_weight == &"" {
                            ""
                        } else {
                            feat_weight.as_str()
                        },
                        if font_style == &"" {
                            ""
                        } else {
                            feat_style.as_str()
                        },
                        feat_size
                    )
                    .unwrap();

                    writeln!(
                        &mut font_rs,
                        r#"include!(concat!(env!("OUT_DIR"), "/generated/{font_name}.rs"));"#
                    )
                    .unwrap();
                    eprintln!();
                }
            }
        }
    }

    write(font_gen_dir.join("fonts").with_extension("rs"), font_rs)
        .expect("Unable to write fonts.rs");
}
