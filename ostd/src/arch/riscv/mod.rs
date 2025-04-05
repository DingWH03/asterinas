// SPDX-License-Identifier: MPL-2.0

//! Platform-specific code for the RISC-V platform.

pub mod boot;
pub(crate) mod cpu;
pub mod device;
pub mod iommu;
pub(crate) mod irq;
pub(crate) mod mm;
pub(crate) mod pci;
pub mod qemu;
pub mod serial;
pub mod task;
pub mod timer;
pub mod trap;

use core::sync::atomic::Ordering;

#[macro_export]
macro_rules! if_tdx_enabled {
    // Match when there is an else block
    ($if_block:block else $else_block:block) => {{
        // 直接返回 else_block（因为只在 RISC-V 上执行）
        $else_block
    }};
    
    // Match when there is no else block
    ($if_block:block) => {{
        // 直接不执行任何操作，返回空
    }};
}


#[cfg(feature = "cvm_guest")]
pub(crate) fn init_cvm_guest() {
    // Unimplemented, no-op
}

pub(crate) unsafe fn late_init_on_bsp() {
    // SAFETY: this function is only called once on BSP.
    unsafe {
        trap::init(true);
    }
    irq::init();

    // SAFETY: we're on the BSP and we're ready to boot all APs.
    unsafe { crate::boot::smp::boot_all_aps() };

    timer::init();
    let _ = pci::init();
}

use core::time::Duration;
use riscv::register::time;

pub fn read_random() -> Option<u64> {
    // 获取当前时间戳（以时钟滴答为单位）
    let time_ticks = time::read();

    // 这里使用一个简单的伪随机算法（线性同余生成器）
    const A: u64 = 6364136223846793005;
    const C: u64 = 1;
    const M: u64 = 0xFFFFFFFFFFFFFFFF; // 最大的 u64 值 (2^64 - 1)

    let mut seed = time_ticks as u64;
    
    // 简单的线性同余生成器公式
    seed = (A.wrapping_mul(seed).wrapping_add(C)) % M;

    // 返回生成的随机数
    Some(seed)
}

pub(crate) unsafe fn init_on_ap() {
    unimplemented!()
}

pub(crate) fn interrupts_ack(irq_number: usize) {
    unimplemented!()
}

/// Return the frequency of TSC. The unit is Hz.
pub fn tsc_freq() -> u64 {
    timer::TIMEBASE_FREQ.load(Ordering::Relaxed)
}

/// Reads the current value of the processor’s time-stamp counter (TSC).
pub fn read_tsc() -> u64 {
    riscv::register::time::read64()
}

pub(crate) fn enable_cpu_features() {
    unsafe {
        riscv::register::sstatus::set_fs(riscv::register::sstatus::FS::Clean);
    }
}
