// SPDX-License-Identifier: BSD-3-Clause
// This module ingests the EFI image file, and reads the `.pdata` and `.rdata` sections
// to extract the embedded unwinding tables.
// see: https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#the-pdata-section
// and: https://learn.microsoft.com/en-us/cpp/build/exception-handling-x64?view=msvc-170

use core::{ffi::c_void, fmt};
use std::{collections::BTreeMap, sync::OnceLock};

use goblin::pe::{PE, exception};
use tracing::debug;
use uefi::{boot, cstr16, fs};

use crate::platform;
#[derive(Clone)]
pub struct UnwindEntry {
    start: usize,
    end: usize,
    prolog: u8,
    codes: Vec<exception::UnwindCode>,
    name: Option<String>,
}

impl UnwindEntry {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn prolog(&self) -> u8 {
        self.prolog
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    pub fn codes(&self) -> &Vec<exception::UnwindCode> {
        &self.codes
    }

    fn relocate(&self, base: usize) -> Self {
        let mut relocated = self.clone();

        relocated.start += base;
        relocated.end += base;

        relocated
    }
}

impl fmt::Display for UnwindEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {:#018x}-{:#018x}\n",
            self.name.clone().unwrap_or("<UNNAMED>".to_string()),
            self.start,
            self.end
        )
    }
}

impl fmt::Debug for UnwindEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UnwindEntry {{ start: {:#018x}, end: {:#018x}, prolog: {}, codes: {:?}, name: {:?} }}",
            self.start, self.end, self.prolog, self.codes, self.name
        )
    }
}

impl Eq for UnwindEntry {}
impl Ord for UnwindEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.end <= other.start {
            std::cmp::Ordering::Less
        } else if other.end <= self.start {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

impl PartialEq for UnwindEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.start == other.start) && (self.end == other.end)
    }
}

impl PartialOrd for UnwindEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.end <= other.start {
            Some(std::cmp::Ordering::Less)
        } else if other.end <= self.start {
            Some(std::cmp::Ordering::Greater)
        } else if self == other {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}

impl PartialEq<usize> for UnwindEntry {
    fn eq(&self, addr: &usize) -> bool {
        (self.start <= *addr) || (*addr < self.end)
    }
}

impl PartialOrd<usize> for UnwindEntry {
    fn partial_cmp(&self, addr: &usize) -> Option<std::cmp::Ordering> {
        if self.end <= *addr {
            Some(std::cmp::Ordering::Less)
        } else if *addr < self.start {
            Some(std::cmp::Ordering::Greater)
        } else if self.eq(addr) {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}

pub static UNWIND_TABLE: OnceLock<Vec<UnwindEntry>> = OnceLock::new();

pub static LOAD_ADDR: OnceLock<usize> = OnceLock::new();
pub static RUNTIME_ADDR: OnceLock<usize> = OnceLock::new();

pub fn has_unwind_table() -> bool {
    if let Some(table) = UNWIND_TABLE.get() {
        return table.len() != 0;
    } else {
        return false;
    }
}

unsafe extern "C" {
    fn efi_main(img: *const c_void, syst: *const c_void);
}

pub fn load_unwind_table() -> Result<(), uefi::Error> {
    debug!("Attempting to load Unwind information");

    // Setup the UEFI filesystem stuff
    let fs = boot::get_image_file_system(boot::image_handle())?;
    let mut fs = fs::FileSystem::new(fs);

    // BUG(aki):
    // This won't always be true, but we don't really have a real way to
    // actually get the name/raw image data
    let img_path = cstr16!("EFI\\BOOT\\BOOTx64.efi");

    // Get the image data and parse the PE file
    let img_data = fs
        .read(img_path)
        .map_err(|_| uefi::Error::new(uefi::Status::INVALID_PARAMETER, ()))?;
    let pe_file = PE::parse(&img_data.as_slice()).unwrap();

    let (load_addr, _) = platform::uefi::get_image_info().unwrap();

    RUNTIME_ADDR.get_or_init(|| efi_main as usize - pe_file.entry);
    LOAD_ADDR.get_or_init(|| load_addr);

    debug!("Base Address (run ): {:#018x}", RUNTIME_ADDR.get().unwrap());
    debug!("Base Address (load): {:#018x}", LOAD_ADDR.get().unwrap());

    // Extract the `.text` virtual address so we can offset symbols to match unwind entries
    let txt_virt = (pe_file.sections)
        .iter()
        .filter(|s| s.name().unwrap() == ".text")
        .nth(0)
        .unwrap()
        .virtual_address;
    // Pull out the string table and the symbol table
    let strtab = pe_file
        .header
        .coff_header
        .strings(&img_data.as_slice())
        .unwrap()
        .unwrap();
    let symbols = pe_file
        .header
        .coff_header
        .symbols(&img_data.as_slice())
        .unwrap()
        .unwrap();

    // Build the virtual address -> symbol name map
    let mut sym_map = BTreeMap::new();
    for sym in symbols
        .iter()
        .filter(|&(_, _, sym)| sym.is_function_definition())
    {
        let sym_base = (sym.2.value + txt_virt) as usize;
        let sym_name = rustc_demangle::demangle(sym.2.name(&strtab).unwrap()).to_string();

        sym_map.insert(sym_base, sym_name);
    }

    let exception_data = pe_file.exception_data.unwrap();

    let _ = UNWIND_TABLE.get_or_init(|| {
        let mut tbl: Vec<UnwindEntry> = Vec::new();

        for func in exception_data.functions() {
            if let Ok(f) = func {
                let unwind = exception_data
                    .get_unwind_info(f, pe_file.sections.as_slice())
                    .unwrap();

                let start_addr = f.begin_address as usize;
                let end_addr = f.end_address as usize;

                let tbl_entry = UnwindEntry {
                    start: start_addr,
                    end: end_addr,
                    prolog: unwind.size_of_prolog,
                    codes: unwind.unwind_codes().filter_map(|f| f.ok()).collect(),
                    name: sym_map.get(&start_addr).cloned(),
                };

                let tbl_run = tbl_entry.relocate(*RUNTIME_ADDR.get().unwrap());
                let tbl_load = tbl_entry.relocate(*LOAD_ADDR.get().unwrap());

                tbl.push(tbl_run);
                tbl.push(tbl_load);
            }
        }

        // We need to half the number of entries because we make 2, for each frame
        debug!("Found {} unwinding table entries", tbl.len() / 2);

        tbl.shrink_to_fit();
        tbl.sort();
        tbl
    });

    Ok(())
}

pub fn unwind_entry_for<'a>(addr: usize) -> Option<&'a UnwindEntry> {
    let uw_tbl = UNWIND_TABLE.get()?;

    if let Ok(idx) = uw_tbl.binary_search_by(|v| v.partial_cmp(&addr).unwrap()) {
        uw_tbl.get(idx)
    } else {
        None
    }
}
