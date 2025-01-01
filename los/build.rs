use std::{env, fs, path::PathBuf};

use std::fs::{read_dir, File};
use std::io::{Result, Write};

fn main() {
    let linker = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(linker, LINK_SCRIPT).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOG");
    println!("cargo:rustc-link-arg=-T{}", linker.display());

    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    insert_program_data().unwrap();
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

static TARGET_PATH: &str = "target/riscv64gc-unknown-none-elf/release/";

fn insert_program_data() -> Result<()> {
    let mut f = File::create("src/link_program.S").unwrap();
    let mut programs: Vec<_> = read_dir("../user/src/bin")
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    programs.sort();

    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_program
_num_program:
    .quad {}"#,
        programs.len()
    )?;

    for i in 0..programs.len() {
        writeln!(f, r#"    .quad program_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad program_{}_end"#, programs.len() - 1)?;

    for (idx, program) in programs.iter().enumerate() {
        println!("program_{}: {}", idx, program);
        writeln!(
            f,
            r#"
    .section .data
    .global program_{0}_start
    .global program_{0}_end
program_{0}_start:
    .incbin "{2}{1}.bin"
program_{0}_end:"#,
            idx, program, TARGET_PATH
        )?;
    }
    Ok(())
}
