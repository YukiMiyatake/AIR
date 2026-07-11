# Linux x86_64 freestanding entry: call AIR `main`, then SYS_exit.
# Linked with: cc -nostdlib -static -no-pie …
    .text
    .globl _start
    .type _start, @function
_start:
    call main
    movl %eax, %edi
    movl $60, %eax
    syscall

    .section .note.GNU-stack,"",@progbits
