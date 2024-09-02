// use std::{env, fs, io::Write, path::PathBuf};

// const BASE_ADDRESS: usize = 0x80400000;
// const STEP: usize = 0x2000;

fn main() {
    // let linker = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("user_linker.ld");
    // let appid = 1;
    // fs::write(linker, format!("BASE_ADDRESS = 0x{:016x};\n", BASE_ADDRESS + appid * STEP)).unwrap();
    // let mut file = fs::OpenOptions::new().append(true).open(linker).unwrap();
    // file.write(LINK_SCRIPT).unwrap();
    // fs::(linker, LINK_SCRIPT).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rustc-link-arg=-T{}", "user/src/linker.ld");
}

// const LINK_SCRIPT: &[u8] = b"
// OUTPUT_ARCH(riscv)
// ENTRY(_start)
// BASE_ADDRESS = 0x80400000;

// SECTIONS
// {
//     . = BASE_ADDRESS;
//     __user_start = .;

//     __text_start = .;
//     .text : {
//         *(.text.entry)
//         *(.text .text.*)
//     }
//     . = ALIGN(4K);
//     __text_end = .;

//     __rodata_start = .;
//     .rodata : {
//         *(.rodata .rodata.*)
//         *(.srodata .srodata.*)
//     }
//     . = ALIGN(4K);
//     __rodata_end = .;

//     __data_start = .;
//     .data : {
//         *(.data .data.*)
//         *(.sdata .sdata.*)
//     }
//     . = ALIGN(4K);
//     __data_end = .;

//     .bss : {
//         *(.bss.stack)
//         __bss_start = .;
//         *(.bss .bss.*)
//         *(.sbss .sbss.*)
//     }

//     __bss_end = .;
//     __user_end = .;
// }";