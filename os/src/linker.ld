OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)

        /* 对齐到 4K，给 trampoline 使用 */
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline);

        . = ALIGN(4K);
        *(.text .text.*)
    }
    /* 对齐到 4K，给 etext  使用，etext 和 srodata 地址是同一个 */
    . = ALIGN(4K);
    etext = .;

    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    erodata = .;

    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    edata = .;

    sbss_with_stack = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    ebss = .;

    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}