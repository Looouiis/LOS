
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
    .incbin "target/riscv64gc-unknown-none-elf/release/initproc"
program_0_end:

    .section .data
    .global program_1_start
    .global program_1_end
program_1_start:
    .incbin "target/riscv64gc-unknown-none-elf/release/usershell"
program_1_end:

.global _program_names
_program_names:
    .string "initproc"
    .string "usershell"
