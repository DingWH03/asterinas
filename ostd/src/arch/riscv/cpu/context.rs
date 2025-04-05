// SPDX-License-Identifier: MPL-2.0

//! CPU execution context control.

use core::{arch::asm, fmt::Debug, sync::atomic::{AtomicBool, Ordering}};

use riscv::register::scause::{Exception, Trap};

pub use crate::arch::riscv::trap::GeneralRegs as RawGeneralRegs;
use crate::{
    arch::riscv::trap::{TrapFrame, UserContext as RawUserContext},
    user::{ReturnReason, UserContextApi, UserContextApiInternal},
};

// 定义FPU寄存器组（兼容F/D扩展）
#[repr(C)]
#[derive(Debug)]
pub struct FpuState {
    pub f: [usize; 32], // f0-f31（根据ABI可能需用u64类型）
    pub fcsr: usize,    // 浮点控制状态寄存器
    dirty: AtomicBool,       // 惰性保存标记
}

impl Clone for FpuState {
    fn clone(&self) -> Self {
        // 读取当前原子值并创建新实例
        let current_dirty = self.dirty.load(Ordering::Relaxed);
        
        FpuState {
            f: self.f.clone(),       // 数组默认支持 Clone
            fcsr: self.fcsr,         // u32 是 Copy
            dirty: AtomicBool::new(current_dirty), // 显式初始化新 AtomicBool
        }
    }
}

impl Default for FpuState {
    fn default() -> Self {
        Self {
            f: [0; 32],
            fcsr: 0,
            dirty: AtomicBool::new(true),
        }
    }
}

impl FpuState {
    pub fn save(&self) {
        unsafe {
            if self.dirty.load(Ordering::Relaxed) {
                let ptr = self as *const Self as *mut Self;
                asm!(
                    // 保存所有浮点寄存器 f0-f31
                    "
                fsd f0, 0*8({0})
                fsd f1, 1*8({0})
                fsd f2, 2*8({0})
                fsd f3, 3*8({0})
                fsd f4, 4*8({0})
                fsd f5, 5*8({0})
                fsd f6, 6*8({0})
                fsd f7, 7*8({0})
                fsd f8, 8*8({0})
                fsd f9, 9*8({0})
                fsd f10, 10*8({0})
                fsd f11, 11*8({0})
                fsd f12, 12*8({0})
                fsd f13, 13*8({0})
                fsd f14, 14*8({0})
                fsd f15, 15*8({0})
                fsd f16, 16*8({0})
                fsd f17, 17*8({0})
                fsd f18, 18*8({0})
                fsd f19, 19*8({0})
                fsd f20, 20*8({0})
                fsd f21, 21*8({0})
                fsd f22, 22*8({0})
                fsd f23, 23*8({0})
                fsd f24, 24*8({0})
                fsd f25, 25*8({0})
                fsd f26, 26*8({0})
                fsd f27, 27*8({0})
                fsd f28, 28*8({0})
                fsd f29, 29*8({0})
                fsd f30, 30*8({0})
                fsd f31, 31*8({0})
                
                // 保存 fcsr 控制寄存器
                csrr t0, fcsr
                sd t0, 32*8({0})
                ",
                    in(reg) (*ptr).f.as_mut_ptr(),
                    out("t0") _,  // 声明 t0 被修改
                    options(nostack, preserves_flags)
                );
                // 更新脏标记
                self.dirty.store(false, Ordering::Relaxed);
            }
        }
    }

    pub fn restore(&self) {
        unsafe {
            let ptr = self as *const Self;
            asm!(
                // 恢复所有浮点寄存器 f0-f31
                "
            fld f0, 0*8({0})
            fld f1, 1*8({0})
            fld f2, 2*8({0})
            fld f3, 3*8({0})
            fld f4, 4*8({0})
            fld f5, 5*8({0})
            fld f6, 6*8({0})
            fld f7, 7*8({0})
            fld f8, 8*8({0})
            fld f9, 9*8({0})
            fld f10, 10*8({0})
            fld f11, 11*8({0})
            fld f12, 12*8({0})
            fld f13, 13*8({0})
            fld f14, 14*8({0})
            fld f15, 15*8({0})
            fld f16, 16*8({0})
            fld f17, 17*8({0})
            fld f18, 18*8({0})
            fld f19, 19*8({0})
            fld f20, 20*8({0})
            fld f21, 21*8({0})
            fld f22, 22*8({0})
            fld f23, 23*8({0})
            fld f24, 24*8({0})
            fld f25, 25*8({0})
            fld f26, 26*8({0})
            fld f27, 27*8({0})
            fld f28, 28*8({0})
            fld f29, 29*8({0})
            fld f30, 30*8({0})
            fld f31, 31*8({0})
            
            // 恢复 fcsr 控制寄存器
            ld t0, 32*8({0})
            csrw fcsr, t0
            ",
                in(reg) ptr,
                out("t0") _,
                options(nostack, preserves_flags)
            );
            self.dirty.store(true, Ordering::Relaxed);
        }
        
    }
}

/// Cpu context, including both general-purpose registers and FPU state.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct UserContext {
    user_context: RawUserContext,
    trap: Trap,
    fpu_state: FpuState, // TODO
    cpu_exception_info: CpuExceptionInfo,
}

/// CPU exception information.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CpuExceptionInfo {
    /// The type of the exception.
    pub code: Exception,
    /// The error code associated with the exception.
    pub page_fault_addr: usize,
    pub error_code: usize, // TODO
}

impl Default for UserContext {
    fn default() -> Self {
        UserContext {
            user_context: RawUserContext::default(),
            trap: Trap::Exception(Exception::Unknown),
            fpu_state: FpuState::default(),
            cpu_exception_info: CpuExceptionInfo::default(),
        }
    }
}

impl Default for CpuExceptionInfo {
    fn default() -> Self {
        CpuExceptionInfo {
            code: Exception::Unknown,
            page_fault_addr: 0,
            error_code: 0,
        }
    }
}

impl CpuExceptionInfo {
    /// Get corresponding CPU exception
    pub fn cpu_exception(&self) -> CpuException {
        self.code
    }
}

impl UserContext {
    /// Returns a reference to the general registers.
    pub fn general_regs(&self) -> &RawGeneralRegs {
        &self.user_context.general
    }

    /// Returns a mutable reference to the general registers
    pub fn general_regs_mut(&mut self) -> &mut RawGeneralRegs {
        &mut self.user_context.general
    }

    /// Returns the trap information.
    pub fn trap_information(&self) -> &CpuExceptionInfo {
        &self.cpu_exception_info
    }

    /// Returns a reference to the FPU state.
    pub fn fpu_state(&self) -> &FpuState {
        &self.fpu_state
    }

    /// Returns a mutable reference to the FPU state.
    pub fn fpu_state_mut(&mut self) -> &mut FpuState {
        &mut self.fpu_state
    }

    /// Sets thread-local storage pointer.
    pub fn set_tls_pointer(&mut self, tls: usize) {
        self.set_tp(tls)
    }

    /// Gets thread-local storage pointer.
    pub fn tls_pointer(&self) -> usize {
        self.tp()
    }

    /// Activates thread-local storage pointer on the current CPU.
    pub fn activate_tls_pointer(&self) {
        // No-op
    }
}

impl UserContextApiInternal for UserContext {
    fn execute<F>(&mut self, mut has_kernel_event: F) -> ReturnReason
    where
        F: FnMut() -> bool,
    {
        let ret = loop {
            self.user_context.run();
            match riscv::register::scause::read().cause() {
                Trap::Interrupt(_) => todo!(),
                Trap::Exception(Exception::UserEnvCall) => {
                    self.user_context.sepc += 4;
                    break ReturnReason::UserSyscall;
                }
                Trap::Exception(e) => {
                    let stval = riscv::register::stval::read();
                    log::trace!("Exception, scause: {e:?}, stval: {stval:#x?}");
                    self.cpu_exception_info = CpuExceptionInfo {
                        code: e,
                        page_fault_addr: stval,
                        error_code: 0,
                    };
                    break ReturnReason::UserException;
                }
            }

            if has_kernel_event() {
                break ReturnReason::KernelEvent;
            }
        };

        crate::arch::irq::enable_local();
        ret
    }

    fn as_trap_frame(&self) -> TrapFrame {
        TrapFrame {
            general: self.user_context.general,
            sstatus: self.user_context.sstatus,
            sepc: self.user_context.sepc,
        }
    }
}

impl UserContextApi for UserContext {
    fn trap_number(&self) -> usize {
        todo!()
    }

    fn trap_error_code(&self) -> usize {
        todo!()
    }

    fn instruction_pointer(&self) -> usize {
        self.user_context.sepc
    }

    fn set_instruction_pointer(&mut self, ip: usize) {
        self.user_context.set_ip(ip);
    }

    fn stack_pointer(&self) -> usize {
        self.user_context.get_sp()
    }

    fn set_stack_pointer(&mut self, sp: usize) {
        self.user_context.set_sp(sp);
    }
}

macro_rules! cpu_context_impl_getter_setter {
    ( $( [ $field: ident, $setter_name: ident] ),*) => {
        impl UserContext {
            $(
                #[doc = concat!("Gets the value of ", stringify!($field))]
                #[inline(always)]
                pub fn $field(&self) -> usize {
                    self.user_context.general.$field
                }

                #[doc = concat!("Sets the value of ", stringify!($field))]
                #[inline(always)]
                pub fn $setter_name(&mut self, $field: usize) {
                    self.user_context.general.$field = $field;
                }
            )*
        }
    };
}

cpu_context_impl_getter_setter!(
    [ra, set_ra],
    [sp, set_sp],
    [gp, set_gp],
    [tp, set_tp],
    [t0, set_t0],
    [t1, set_t1],
    [t2, set_t2],
    [s0, set_s0],
    [s1, set_s1],
    [a0, set_a0],
    [a1, set_a1],
    [a2, set_a2],
    [a3, set_a3],
    [a4, set_a4],
    [a5, set_a5],
    [a6, set_a6],
    [a7, set_a7],
    [s2, set_s2],
    [s3, set_s3],
    [s4, set_s4],
    [s5, set_s5],
    [s6, set_s6],
    [s7, set_s7],
    [s8, set_s8],
    [s9, set_s9],
    [s10, set_s10],
    [s11, set_s11],
    [t3, set_t3],
    [t4, set_t4],
    [t5, set_t5],
    [t6, set_t6]
);

/// CPU exception.
pub type CpuException = Exception;
