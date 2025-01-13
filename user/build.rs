use std::{env, fs, path::PathBuf};

fn main() {
    let linker = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("user_linker.ld");
    fs::write(linker, LINK_SCRIPT).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rustc-link-arg=-T{}", linker.display());
}

const LINK_SCRIPT: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x10000;

SECTIONS
{
    . = BASE_ADDRESS;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
    }
    __bss_start = .;
    .bss : {
        *(.bss .bss.*)
    }
    __bss_end = .;
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }
}";
