use std::{env, fs, path::PathBuf};

fn main() {
    let linker = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(linker, LINK_SCRIPT).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rustc-link-arg=-T{}", linker.display());
}

const LINK_SCRIPT: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    __kernel_start = .;

    __text_start = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    __text_end = .;

    __rodata_start = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    __rodata_end = .;

    __data_start = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    __data_end = .;

    .bss : {
        *(.bss.stack)
        __bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    __bss_end = .;
    __kernel_end = .;
}";