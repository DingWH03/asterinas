// SPDX-License-Identifier: MPL-2.0

//! RISC-V Boot Code
//! 初始化页表、栈以及 MMU，功能等效于汇编版本（修正 Sv48 映射）

use riscv::register::satp;
use core::arch::asm;

const TASK_STACK_SIZE: usize = 0x40000;
/// 物理地址到虚拟地址的偏移量（用于内核高地址映射修正）
const PHYS_VIRT_OFFSET: usize = 0xffff_ffc0_0000_0000;

/// 引导栈：用于存储内核启动栈
#[link_section = ".bss.stack"]
static mut BOOT_STACK: [u8; TASK_STACK_SIZE] = [0; TASK_STACK_SIZE];

/// 一级页表（根页表），共 512 项
#[link_section = ".data.boot_page_table"]
static mut BOOT_PT_L1: [u64; 512] = [0; 512];

/// 二级页表，采用单独段存放
#[link_section = ".data.boot_page_table_2nd"]
static mut BOOT_PT_L2: [u64; 512] = [0; 512];

/// 初始化引导页表（修正权限和地址计算）
#[no_mangle]
#[allow(clippy::identity_op)]
unsafe fn init_boot_page_table() {
    // --- 修正页表项权限 ---
    // 0x00000: 数据段 RW- (V=1, R=1, W=1, A=1, D=1) -> 0xcf → 修正为 0x0f（RW-）
    // 0x80000: 代码段 R-X (V=1, R=1, X=1, A=1, D=1) → 0x1f
    BOOT_PT_L1[0] = (0x0 << 10) | 0x0f; // RW-
    BOOT_PT_L1[256] = (0x0 << 10) | 0x0f; // RW-

    // --- 修正二级页表地址计算 ---
    let pt2_phys = BOOT_PT_L2.as_ptr() as usize;
    // PPN = pt2_phys >> 12，标志位 V=1 → 0x01
    BOOT_PT_L1[511] = ((pt2_phys >> 12) as u64) | 0x01;

    // --- 修正二级页表权限 ---
    // 0x00000: 数据段 RW- (0x0f)
    // 0x40000: 数据段 RW- (0x0f)
    // 0x80000: 代码段 R-X (0x1f)
    BOOT_PT_L2[508] = (0x00000 << 10) | 0x0f; // RW-
    BOOT_PT_L2[509] = (0x40000 << 10) | 0x0f; // RW-
    BOOT_PT_L2[510] = (0x80000 << 10) | 0x1f; // R-X
}

/// 初始化 MMU（修正 satp 设置）
#[no_mangle]
unsafe fn init_mmu() {
    // let pt1_phys = BOOT_PT_L1.as_ptr() as usize;
    // // Sv48 MODE = 9 << 60, PPN = pt1_phys >> 12
    // let satp_value = (9 << 60) | (pt1_phys >> 2);
    // satp::set(satp::Mode::Sv48, 0, satp_value as usize);
    asm!(
        "
        la     t0, {boot_pagetable}
        li     t1, 9 << 60
        srli   t0, t0, 12
        or     t0, t0, t1
        csrw   satp, t0
        sfence.vma",
        boot_pagetable = sym BOOT_PT_L1
    );
}

/// Rust 启动入口（修正地址跳转逻辑）
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    asm!(   
        "
        # 保存 hartid 和 dtb 指针
        mv      s0, a0                  # 保存 hartid
        mv      s1, a1                  # 保存 dtb 指针
        
        # 设置栈指针（物理地址）
        la      sp, {boot_stack}        # 加载栈基地址
        li      t0, {boot_stack_size}   # 栈大小
        add     sp, sp, t0              # 定位到栈顶

        # 初始化页表和 MMU
        call    {init_boot_page_table}
        call    {init_mmu}

        # 栈指针切换到虚拟地址（已映射）
        li      t0, {phys_virt_offset}
        add     sp, sp, t0

        # 跳转到 Rust 内核入口（直接使用虚拟地址，无需手动修正）
        mv      a0, s0                # hartid
        mv      a1, s1                # dtb
        la      a2, {riscv_boot}      # 已映射到高地址
        jalr    a2
        ",
        boot_stack = sym BOOT_STACK,
        boot_stack_size = const TASK_STACK_SIZE,
        phys_virt_offset = const PHYS_VIRT_OFFSET,
        init_boot_page_table = sym init_boot_page_table,
        init_mmu = sym init_mmu,
        riscv_boot = sym super::riscv_boot,
        options(noreturn)
    );
}

/// 二级 CPU 启动（修正地址处理）
#[cfg(feature = "smp")]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start_secondary() -> ! {
    asm!(
        "
        # 保存 hartid 和栈指针（物理地址）
        mv      s0, a0                  # hartid
        mv      sp, a1                  # 使用传入的栈指针
        
        # 启用 MMU
        call    {init_mmu}

        # 栈指针切换到虚拟地址
        li      t0, {phys_virt_offset}
        add     sp, sp, t0

        # 跳转到二级核心入口（直接使用虚拟地址）
        mv      a0, s0
        la      a1, {rust_entry_secondary}
        jalr    a1
        ",
        phys_virt_offset = const PHYS_VIRT_OFFSET,
        init_mmu = sym init_mmu,
        rust_entry_secondary = sym super::rust_entry_secondary,
        options(noreturn)
    );
}