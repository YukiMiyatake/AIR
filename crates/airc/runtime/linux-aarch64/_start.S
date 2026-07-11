# Linux aarch64 freestanding entry: call AIR `main`, then SYS_exit.
# Linked with: cc -nostdlib -static -no-pie …
    .text
    .globl _start
    .type _start, @function
_start:
    bl main
    mov w8, #93
    svc #0

    .section .note.GNU-stack,"",@progbits
