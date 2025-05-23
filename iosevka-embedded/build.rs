// SPDX-License-Identifier: BSD-3-Clause

use eg_font_converter::{FontConverter, Mapping};
// use rc_zip_sync::ReadZip;
use std::fmt::Write;
use std::fs::{self, write};
// use std::fs::File;
// use std::io::Cursor;
use std::path::{Path, PathBuf};
// use std::process::Command;
use std::env;
// use std::io;

// fn rasterize_font(input_font: &Path, bdf_output: &Path, size: u8) {
//     // Yes, I know how bad this is
//     let ff_cmd = format!(
//         "Open(\"{}\"); BitmapsAvail([{}]); BitmapsRegen([{1}]); Generate(\"{}{}.\", \"bdf\")",
//         input_font.to_str().unwrap(),
//         size,
//         bdf_output.parent().unwrap().to_str().unwrap(),
//         bdf_output.file_stem().unwrap().to_str().unwrap()
//     );
//
//     let ret = Command::new("fontforge")
//         .current_dir(env::current_dir().unwrap())
//         .arg("-lang=ff")
//         .arg(format!("-c '{}'", ff_cmd))
//         .status()
//         .expect("Unable to rasterize font!");
//
//     assert!(ret.success());
// }
//
// fn download_font_archive(url: String, file: &Path) -> Result<(), Box<dyn std::error::Error>> {
//     let resp = reqwest::blocking::get(url)?;
//     let mut f = File::create(file)?;
//     let mut data = Cursor::new(resp.bytes()?);
//     io::copy(&mut data, &mut f)?;
//     Ok(())
// }

// const FONT_URL: &str = "https://github.com/be5invis/Iosevka/releases/download/v33.2.1/";
// const FONT_ARCHIVE: &str = "PkgTTF-IosevkaFixed-33.2.1.zip";
const FONTS: &[&str] = &["IosevkaFixed-Extended"];
const FONT_SIZES: &[u8] = &[8, 16, 24, 32];
const FONT_STYLES: &[&str] = &[
    "", // No Style
    "Bold", "Thin",
];

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let font_bdf_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts"));
    let output_dir = PathBuf::from(&std::env::var_os("OUT_DIR").expect("no OUT_DIR"));

    // Compose all the font sets we want to generate
    let font_sets = FONTS
        .iter()
        .flat_map(|f| FONT_STYLES.iter().map(move |s| format!("{f}{s}")))
        .flat_map(|fs| FONT_SIZES.iter().map(move |s| (fs.clone(), s)));

    // let font_archive = output_dir.join(FONT_ARCHIVE);
    // if !fs::exists(&font_archive).expect("Unable to resolve path") {
    //     eprintln!("Downloading font archive");
    //     download_font_archive(
    //         format!("{}/{}", FONT_URL, FONT_ARCHIVE),
    //         font_archive.as_path(),
    //     )
    //     .expect("Unable to download font archive");
    // }

    //     let font_ttf_dir = output_dir.join("ttf");
    //     if !fs::exists(&font_ttf_dir).expect("Unable to resolve path") {
    //         fs::create_dir(&font_ttf_dir).expect("Unable to make ttf font directory");
    //
    //         let archive_bytes = fs::read(font_archive).unwrap();
    //         let arch_slice = &archive_bytes[..];
    //         let archive = arch_slice.read_zip().unwrap();
    //     }

    let font_gen_dir = output_dir.join("generated");
    if !fs::exists(&font_gen_dir).expect("Unable to resolve path") {
        fs::create_dir(&font_gen_dir).expect("Unable to make generated font directory");
    }

    let mut font_rs = String::new();
    for font in font_sets {
        let bdf_font_name = format!("{}-{}", font.0, font.1);
        let bdf_font = font_bdf_dir
            .join(bdf_font_name.clone())
            .with_extension("bdf");
        eprintln!("Converting {}...", bdf_font.to_str().unwrap());
        println!("cargo::rerun-if-changed={}", bdf_font.to_str().unwrap());

        let font_name = bdf_font_name.as_str().replace("-", "_").to_lowercase();

        let output = FontConverter::new(&bdf_font, &font_name.to_uppercase())
            .missing_glyph_substitute('*')
            .glyphs(Mapping::Ascii)
            // Smooth boxes
            .glyphs('█'..='▏')
            // Misc
            .glyphs(&['‼', '‽', '⁇', '⁈', '⁉', '№', '⍉', '➜'][..])
            // Box drawing
            .glyphs('╭'..='╰')
            .glyphs(&['─', '│', '├', '┤', '┬', '┴'][..])
            .convert_eg_bdf()
            .expect("Unable to convert BDF font");

        let outfile_rs = font_gen_dir.join(font_name.clone()).with_extension("rs");
        let outfile_data = font_gen_dir.join(font_name.clone()).with_extension("data");

        eprintln!("Writing {}", outfile_rs.to_str().unwrap());
        write(outfile_rs, output.rust()).expect("Unable to write output rust file");
        eprintln!("Writing {}", outfile_data.to_str().unwrap());
        write(outfile_data, output.data()).expect("Unable to write output data file");

        writeln!(
            &mut font_rs,
            r#"include!(concat!(env!("OUT_DIR"), "/generated/{font_name}.rs"));"#
        )
        .unwrap();
        eprintln!();
    }

    write(font_gen_dir.join("fonts").with_extension("rs"), font_rs)
        .expect("Unable to write fonts.rs");
}
