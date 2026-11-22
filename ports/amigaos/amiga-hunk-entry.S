/* AmigaDOS starts a LoadSeg executable at the first word of its first CODE
 * hunk. Keep this first input hunk as an explicit jump to the Rust runtime. */
    .section .text.hunk_entry,"ax",@progbits
    .globl __amiga_hunk_entry
    .type __amiga_hunk_entry,@function
__amiga_hunk_entry:
    jmp _start
    .size __amiga_hunk_entry, .-__amiga_hunk_entry
