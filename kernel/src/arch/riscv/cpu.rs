// SPDX-License-Identifier: MPL-2.0

use ostd::{
    cpu::context::{CpuExceptionInfo, RawGeneralRegs, UserContext},
    Pod,
};

use alloc::{
    format,
    string::{String, ToString},
    // vec::Vec,
};

use crate::{cpu::LinuxAbi, thread::exception::PageFaultInfo, vm::perms::VmPerms};

impl LinuxAbi for UserContext {
    fn syscall_num(&self) -> usize {
        self.a7()
    }

    fn syscall_ret(&self) -> usize {
        self.a0()
    }

    fn set_syscall_num(&mut self, num: usize) {
        self.set_a7(num);
    }

    fn set_syscall_ret(&mut self, ret: usize) {
        self.set_a0(ret)
    }

    fn syscall_args(&self) -> [usize; 6] {
        [
            self.a0(),
            self.a1(),
            self.a2(),
            self.a3(),
            self.a4(),
            self.a5(),
        ]
    }

    fn set_tls_pointer(&mut self, tls: usize) {
        self.set_tp(tls);
    }

    fn tls_pointer(&self) -> usize {
        self.tp()
    }
}

/// General-purpose registers.
#[derive(Debug, Clone, Copy, Pod, Default)]
#[repr(C)]
pub struct GpRegs {
    pub zero: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

macro_rules! copy_gp_regs {
    ($src: ident, $dst: ident) => {
        $dst.zero = $src.zero;
        $dst.ra = $src.ra;
        $dst.sp = $src.sp;
        $dst.gp = $src.gp;
        $dst.tp = $src.tp;
        $dst.t0 = $src.t0;
        $dst.t1 = $src.t1;
        $dst.t2 = $src.t2;
        $dst.s0 = $src.s0;
        $dst.s1 = $src.s1;
        $dst.a0 = $src.a0;
        $dst.a1 = $src.a1;
        $dst.a2 = $src.a2;
        $dst.a3 = $src.a3;
        $dst.a4 = $src.a4;
        $dst.a5 = $src.a5;
        $dst.a6 = $src.a6;
        $dst.a7 = $src.a7;
        $dst.s2 = $src.s2;
        $dst.s3 = $src.s3;
        $dst.s4 = $src.s4;
        $dst.s5 = $src.s5;
        $dst.s6 = $src.s6;
        $dst.s7 = $src.s7;
        $dst.s8 = $src.s8;
        $dst.s9 = $src.s9;
        $dst.s10 = $src.s10;
        $dst.s11 = $src.s11;
        $dst.t3 = $src.t3;
        $dst.t4 = $src.t4;
        $dst.t5 = $src.t5;
        $dst.t6 = $src.t6;
    };
}

impl GpRegs {
    pub fn copy_to_raw(&self, dst: &mut RawGeneralRegs) {
        copy_gp_regs!(self, dst);
    }

    pub fn copy_from_raw(&mut self, src: &RawGeneralRegs) {
        copy_gp_regs!(src, self);
    }
}

impl TryFrom<&CpuExceptionInfo> for PageFaultInfo {
    // [`Err`] indicates that the [`CpuExceptionInfo`] is not a page fault,
    // with no additional error information.
    type Error = ();

    fn try_from(value: &CpuExceptionInfo) -> Result<Self, ()> {
        use riscv::register::scause::Exception;

        let required_perms = match value.cpu_exception() {
            Exception::InstructionPageFault => VmPerms::EXEC,
            Exception::LoadPageFault => VmPerms::READ,
            Exception::StorePageFault => VmPerms::WRITE,
            _ => return Err(()),
        };

        Ok(PageFaultInfo {
            address: value.page_fault_addr,
            required_perms,
        })
    }
}

pub struct CpuInfo {
    pub processor: u32,
    pub vendor_id: String,
    pub cpu_family: u32,
    pub model: u32,
    pub model_name: String,
    pub stepping: u32,
    pub microcode: u32,
    pub cpu_mhz: u32,
    pub cache_size: u32,      // 以字节为单位
    pub tlb_size: u32,        // 4K 页数量
    pub physical_id: u32,
    pub siblings: u32,
    pub core_id: u32,
    pub cpu_cores: u32,
    pub apicid: u32,
    pub initial_apicid: u32,
    pub cpuid_level: u32,
    pub flags: String,
    pub bugs: String,
    pub clflush_size: u8,
    pub cache_alignment: u32,
    pub address_sizes: String,
    pub power_management: String,
}

impl CpuInfo {
    pub fn new(processor_id: u32) -> Self {
        Self {
            processor: processor_id,
            vendor_id: Self::get_vendor_id(),
            cpu_family: Self::get_cpu_family(),
            model: Self::get_model(),
            model_name: Self::get_model_name(),
            stepping: Self::get_stepping(),
            microcode: Self::get_microcode(),
            cpu_mhz: Self::get_clock_speed().unwrap_or(0),
            cache_size: Self::get_cache_size().unwrap_or(0),
            tlb_size: Self::get_tlb_size().unwrap_or(0),
            physical_id: Self::get_physical_id().unwrap_or(0),
            siblings: Self::get_siblings_count().unwrap_or(0),
            core_id: Self::get_core_id(),
            cpu_cores: Self::get_cpu_cores(),
            apicid: Self::get_apicid(),
            initial_apicid: Self::get_initial_apicid(),
            cpuid_level: Self::get_cpuid_level(),
            flags: Self::get_cpu_flags(),
            bugs: Self::get_cpu_bugs(),
            clflush_size: Self::get_clflush_size(),
            cache_alignment: Self::get_cache_alignment(),
            address_sizes: Self::get_address_sizes(),
            power_management: Self::get_power_management(),
        }
    }

    /// 将 CPU 信息格式化成字符串
    pub fn collect_cpu_info(&self) -> String {
        format!(
            "processor\t: {}\n\
             vendor_id\t: {}\n\
             cpu family\t: {}\n\
             model\t\t: {}\n\
             model name\t: {}\n\
             stepping\t: {}\n\
             microcode\t: 0x{:x}\n\
             cpu MHz\t\t: {}\n\
             cache size\t: {} KB\n\
             TLB size\t: {} 4K pages\n\
             physical id\t: {}\n\
             siblings\t: {}\n\
             core id\t\t: {}\n\
             cpu cores\t: {}\n\
             apicid\t\t: {}\n\
             initial apicid\t: {}\n\
             cpuid level\t: {}\n\
             flags\t\t: {}\n\
             bugs\t\t: {}\n\
             clflush size\t: {} bytes\n\
             cache_alignment\t: {} bytes\n\
             address sizes\t: {}\n\
             power management: {}\n",
            self.processor,
            self.vendor_id,
            self.cpu_family,
            self.model,
            self.model_name,
            self.stepping,
            self.microcode,
            self.cpu_mhz,
            self.cache_size / 1024, // 输出为 KB
            self.tlb_size,
            self.physical_id,
            self.siblings,
            self.core_id,
            self.cpu_cores,
            self.apicid,
            self.initial_apicid,
            self.cpuid_level,
            self.flags,
            self.bugs,
            self.clflush_size,
            self.cache_alignment,
            self.address_sizes,
            self.power_management
        )
    }

    fn get_vendor_id() -> String {
        "riscv".to_string()
    }

    fn get_cpu_family() -> u32 {
        0
    }

    fn get_model() -> u32 {
        0
    }

    fn get_stepping() -> u32 {
        0
    }

    fn get_model_name() -> String {
        "RISC-V".to_string()
    }

    fn get_microcode() -> u32 {
        0
    }

    fn get_clock_speed() -> Option<u32> {
        // 返回默认 1000 MHz
        Some(1000)
    }

    /// 返回缓存大小（字节）
    fn get_cache_size() -> Option<u32> {
        // 默认 32 MB（32 * 1024 * 1024 字节）
        Some(32 * 1024 * 1024)
    }

    fn get_tlb_size() -> Option<u32> {
        Some(512)
    }

    fn get_physical_id() -> Option<u32> {
        Some(0)
    }

    fn get_siblings_count() -> Option<u32> {
        Some(1)
    }

    fn get_core_id() -> u32 {
        0
    }

    fn get_cpu_cores() -> u32 {
        1
    }

    fn get_apicid() -> u32 {
        0
    }

    fn get_initial_apicid() -> u32 {
        Self::get_apicid()
    }

    fn get_cpuid_level() -> u32 {
        0
    }

    fn get_cpu_flags() -> String {
        "fpu vme de pse tsc msr pae mce".to_string()
    }

    fn get_cpu_bugs() -> String {
        "".to_string()
    }

    fn get_clflush_size() -> u8 {
        64
    }

    fn get_cache_alignment() -> u32 {
        64
    }

    fn get_address_sizes() -> String {
        "64 bits physical, 64 bits virtual".to_string()
    }

    fn get_power_management() -> String {
        "".to_string()
    }
}