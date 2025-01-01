
    .align 3
    .section .data
    .global _num_program
_num_program:
    .quad 2
    .quad program_0_start
    .quad program_1_start
    .quad program_1_end

    .section .data
    .global program_0_start
    .global program_0_end
program_0_start:
    .incbin "target/riscv64gc-unknown-none-elf/release/a.bin"
program_0_end:

    .section .data
    .global program_1_start
    .global program_1_end
program_1_start:
    .incbin "target/riscv64gc-unknown-none-elf/release/b.bin"
program_1_end:
