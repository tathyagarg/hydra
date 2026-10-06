#![allow(dead_code, non_snake_case)]

use crate::cpu::{
    brbe::{BRBINF_EL1_Type, BRBSRC_EL1_Type, BRBTGT_EL1_Type, Brbe},
    pmu::Pmu,
};

mod brbe;
pub mod common;
mod instructions;
mod pmu;
mod pstate;
mod regs;
mod shared;

const LOWER_32: u64 = 0xFFFF_FFFF_0000_0000;
const REG_SP: usize = 0b11111;

pub type Register = usize;

pub enum CmpOp {
    EQ,
    NE,
    GE,
    GT,
    LT,
    LE,
    UN,
}

pub enum ExtendType {
    UXTB = 0b000,
    UXTH = 0b001,
    UXTW = 0b010,
    /// LSL|UXTX
    UXTX = 0b011,
    SXTB = 0b100,
    SXTH = 0b101,
    SXTW = 0b110,
    SXTX = 0b111,
}

impl ExtendType {
    pub fn decode(n: u8) -> Option<Self> {
        if n > 0b111 {
            None
        } else {
            Some(match n {
                0b000 => Self::UXTB,
                0b001 => Self::UXTH,
                0b010 => Self::UXTW,
                0b011 => Self::UXTX,
                0b100 => Self::SXTB,
                0b101 => Self::SXTH,
                0b110 => Self::SXTW,
                0b111 => Self::SXTX,
                _ => unreachable!("n not in (0b000..=0b111) but also not > 0b111?"),
            })
        }
    }
}

pub enum ShiftType {
    LSL = 0b00,
    LSR = 0b01,
    ASR = 0b10,
    ROR = 0b11,
}

impl ShiftType {
    pub fn decode(n: u8) -> Option<Self> {
        if n > 0b11 {
            None
        } else {
            Some(match n {
                0b00 => Self::LSL,
                0b01 => Self::LSR,
                0b10 => Self::ASR,
                0b11 => Self::ROR,
                _ => unreachable!("n not in (0b00..=0b11) but also not > 0b11?"),
            })
        }
    }
}

pub enum BranchType {
    DIRCALL,
    INDCALL,
    ERET,
    DBGEXIT,
    RET,
    DIR,
    INDIR,
    EXCEPTION,
    RESET,
    UNKNOWN,
}

fn to_uint<const N: usize>(v: u64) -> u64 {
    v & ((1 << N) - 1)
}

pub struct Cpu {
    /// R0-R31: 31 general-purpose registers, R0 to R30. Each can be accessed as:
    /// - A 64-bit general-purpose register named X0 to X30.
    /// - A 32-bit general-purpose register named W0 to W30.
    /// The X30 general-purpose register is used as the procedure call link register.
    x: [u64; 31],

    /// A 64-bit dedicated Stack Pointer register. The least significant 32 bits of the stack pointer can be accessed
    /// using the register name WSP.
    /// The use of SP as an operand in an instruction, indicates the use of the current stack pointer.
    sp: u64,

    /// A 64-bit Program Counter holding the address of the current instruction.
    /// Software cannot write directly to the PC. It can be updated only on a branch, exception entry or exception
    /// return.
    pc: u64,

    /// 32 SIMD&FP registers, V0 to V31. Each can be accessed as:
    /// - A 128-bit register named Q0 to Q31.
    /// - A 64-bit register named D0 to D31.
    /// - A 32-bit register named S0 to S31.
    /// - A 16-bit register named H0 to H31.
    /// - An 8-bit register named B0 to B31.
    /// - A 128-bit vector of elements.
    /// - A 64-bit vector of elements.
    /// Where the number of bits described by a register name does not occupy an entire SIMD&FP register, it
    /// refers to the least significant bits.
    v: [u128; 32],

    /// Purpose:
    ///     Controls behavior of the floating-point instructions.
    ///
    /// Bits [63:27]
    ///     Reserved, Res0
    ///
    /// AHP, bit [26]
    ///     Alternative half-precision control bit.
    ///
    /// AHP  Meaning
    /// 0b0  IEEE half-precision format selected.
    /// 0b1  Alternative half-precision format selected.
    ///
    /// This bit is used only for conversions between half-precision floating-point and other floating-point formats.
    ///     The data-processing instructions added as part of the FEAT_FP16 extension always use the IEEE half-precision
    ///     format, and ignore the value of this bit.
    ///     The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// DN, bit [25]
    ///     Default NaN use for NaN propagation.
    ///
    /// DN   Meaning
    /// 0b0  NaN operands propagate through to the output of a floating-point operation.
    /// 0b1  Any operation involving one or more NaNs returns the default NaN.
    ///      This bit has no effect on the output of the FABS and FNEG instructions. This bit has no effect on the output
    ///      of the FMAX, FMAXP, FMAXV, FMIN, FMINP, and FMINV instructions when FPCR.AH is 1.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// FZ, bit [24]
    ///     Flushing denormalized numbers to zero control bit.
    ///
    /// FZ   Meaning
    /// 0b0  If FPCR.AH is 0, the flushing to zero of single-precision and double-precision denormalized inputs to, and
    ///      outputs of, floating-point instructions not enabled by this control, but other factors might cause the input
    ///      denormalized numbers to be flushed to zero.
    ///      If FPCR.AH is 1, the flushing to zero of single-precision and double-precision denormalized outputs of
    ///      floating-point instructions not enabled by this control, but other factors might cause the input denormalized
    ///      numbers to be flushed to zero.
    /// 0b1  If FPCR.AH is 0, denormalized single-precision and double-precision inputs to, and outputs from,
    ///      floating-point instructions are flushed to zero.
    ///      If FPCR.AH is 1, denormalized single-precision and double-precision outputs from floating-point
    ///      instructions are flushed to zero.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// RMode, bits [23:22]
    ///     Rounding Mode control field.
    ///
    /// Rmode  Meaning
    /// 0b00   Round to Nearest (RN) mode.
    /// 0b01   Round towards Plus Infinity (RP) mode.
    /// 0b10   Round towards Minus Infinity (RM) mode.
    /// 0b11   Round towards Zero (RZ) mode.
    ///
    /// The following instructions ignore this field and use Round to Nearest mode:
    ///     - All FP8 instructions, as described in ‘Summary of FP8 instruction behaviors’.
    ///     - When FPCR.AH is 1, then the floating-point reciprocal estimate, step and exponent instructions, floating-point
    ///       square root estimate and step instructions, and instructions that follow Alternate BFloat16 behaviors, as defined
    ///       in ‘Rounding’.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// Stride, bits [21:20]
    ///     This field has no function in AArch64 state, and nonzero values are ignored during execution in AArch64 state.
    ///     This field is included only for context saving and restoration of the AArch32 FPSCR.Stride field.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// When an implementation implements FPSCR.{Stride, Len} as RAZ, access to this field is RAZ/WI.
    ///
    /// FZ16, bit [19]
    /// When FEAT_FP16 is implemented:
    ///     Flushing denormalized numbers to zero control bit on half-precision data-processing instructions.
    ///
    /// FZ16  Meaning
    /// 0b0   For some instructions, this bit disables flushing to zero of inputs and outputs that are half-precision
    ///       denormalized numbers.
    /// 0b1   Flushing denormalized numbers to zero enabled.
    ///       For some instructions that do not convert a half-precision input to a higher precision output, this bit
    ///       enables flushing to zero of inputs and outputs that are half-precision denormalized numbers.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// Otherwise:
    ///     Reserved, RES0.
    ///
    /// Len, bits [18:16]
    ///     This field has no function in AArch64 state, and nonzero values are ignored during execution in AArch64 state.
    ///     This field is included only for context saving and restoration of the AArch32 FPSCR.Len field.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// When an implementation implements FPSCR.{Stride, Len} as RAZ, access to this field is RAZ/WI.
    ///
    /// IDE, bit [15]
    ///     Input Denormal floating-point exception trap enable.
    ///
    /// IDE  Meaning
    /// 0b0  Untrapped exception handling selected. If the floating-point exception occurs, the
    ///      FPSR.IDC bit is set to 1.
    /// 0b1  Trapped exception handling selected. If the floating-point exception occurs, the PE does not update the
    ///      FPSR.IDC bit.
    ///
    /// When the PE is in Streaming SVE mode, and FEAT_SME_FA64 is not implemented or not enabled, the value of
    ///     FPCR.IDE is treated as 0 for all purposes other than a direct read or write of the FPCR.
    ///     The Effective value of this bit controls both scalar and vector floating-point arithmetic.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// When FEAT_TRAPFPE is not implemented and floating-point exception traps are disabled, access to this field is
    ///     RAZ/WI.
    ///
    /// Bit [14]
    ///     Reserved, RES0.
    ///
    /// EBF, bit [13]
    /// When FEAT_EBF16 is implemented:
    ///     The value of this bit controls the numeric behaviors of BFloat16 dot product calculations performed by the
    ///     BFDOT, BFMMLA, BFMOPA, and BFMOPS instructions. If FEAT_SME2 is implemented, this also controls
    ///     BFVDOT instruction.
    ///
    /// When ID_AA64ISAR1_EL1.BF16 and ID_AA64ZFR0_EL1.BF16 are 0b0010, the PE supports the
    ///     FPCR.EBF field. Otherwise, FPCR.EBF is RES0.
    ///
    /// EBF  Meaning
    /// 0b0  These instructions use the standard BFloat16 behaviors:
    ///      - Ignoring the FPCR.RMode control and using the rounding mode defined for BFloat16. For more
    ///        information, see ‘Round to Odd mode’.
    ///      - Flushing denormalized inputs and outputs to zero, as if the FPCR.FZ and FPCR.FIZ controls had the
    ///        value ‘1’.
    ///      - Performing unfused multiplies and additions with intermediate rounding of all products and sums.
    /// 0b1  These instructions use the extended BFloat16 behaviors:
    ///      - Supporting all four IEEE 754 rounding modes selected by the FPCR.RMode control.
    ///      - Optionally, flushing denormalized inputs and outputs to zero, as governed by the FPCR.FZ and
    ///        FPCR.FIZ controls.
    ///      - Performing a fused two-way sum-of-products for each pair of adjacent BFloat16 elements, without
    ///        intermediate rounding of the products, but rounding the single-precision sum before addition to the
    ///        accumulator.
    ///      - Generating the default NaN as intermediate sum-of-products when any multiplier input is a NaN, or
    ///        any product is infinity × 0.0, or there are infinite products with differing signs.
    ///      - Generating an intermediate sum-of-products of the same infinity when there are infinite products all
    ///        with the same sign.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// Otherwise:
    ///     Reserved, RES0.
    ///
    /// IXE, bit [12]
    ///     Inexact floating-point exception trap enable.
    ///
    /// IXE  Meaning
    /// 0b0  Untrapped exception handling selected. If the floating-point exception occurs, the FPSR.IXC bit is set to 1.
    /// 0b1  Trapped exception handling selected. If the floating-point exception occurs, the PE does not update the FPSR.IXC bit.
    ///
    /// When the PE is in Streaming SVE mode, and FEAT_SME_FA64 is not implemented or not enabled, the value of
    ///     FPCR.IXE is treated as 0 for all purposes other than a direct read or write of the FPCR.
    ///     The Effective value of this bit controls both scalar and vector floating-point arithmetic.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// When FEAT_TRAPFPE is not implemented and floating-point exception traps are disabled, access to this field is
    ///     RAZ/WI.
    ///
    /// UFE, bit [11]
    ///     Underflow floating-point exception trap enable.
    ///
    /// UFE  Meaning
    /// 0b0  Untrapped exception handling selected. If the floating-point exception occurs, the FPSR.UFC bit is set to 1.
    /// 0b1  Trapped exception handling selected. If the floating-point exception occurs and Flush-to-zero is not enabled, the PE does not update the FPSR.UFC bit.
    ///
    /// When the PE is in Streaming SVE mode, and FEAT_SME_FA64 is not implemented or not enabled, the value of
    ///     FPCR.UFE is treated as 0 for all purposes other than a direct read or write of the FPCR.
    ///     The Effective value of this bit controls both scalar and vector floating-point arithmetic.
    ///
    /// The reset behavior of this field is:
    ///     - On a Warm reset, this field resets to an architecturally UNKNOWN value.
    ///
    /// When FEAT_TRAPFPE is not implemented and floating-point exception traps are disabled, access to this field is
    ///     RAZ/WI.
    ///
    /// OFE, bit [10]
    ///     Overflow floating-point exception trap enable.
    ///
    /// TODO: Finish the rest of this (from page 979)
    fcpr: u64,

    /// TODO: Finish the documentation (from page 992)
    fpsr: u64,

    /// NOTE: TCR_EL1[31:0] map to TTBCR[31:0]
    /// TCR_EL1[63:32] map to TTBCR2[31:0]
    tcr_el1: u64,
    tcr_el2: u64,
    tcr_el3: u64,

    scr: u32,
    scr_el3: u64,

    hcr_el2: u64,

    brbfcr_el0: u64,
    brbfcr_el1: u64,

    brbcr_el1: u64,
    brbcr_el2: u64,

    mdcr_el3: u64,

    brbe: Brbe,
    pmu: Pmu,

    pstate: pstate::PState,
}

impl Default for Cpu {
    /// WARN: Use for testing only!!
    fn default() -> Self {
        Self {
            x: [0; 31],
            sp: 0,
            pc: 0,
            v: [0; 32],
            fcpr: 0,
            fpsr: 0,
            tcr_el1: 0,
            tcr_el2: 0,
            tcr_el3: 0,
            scr: 0,
            scr_el3: 0,
            hcr_el2: 0,
            brbfcr_el0: 0,
            brbfcr_el1: 0,
            brbcr_el1: 0,
            brbcr_el2: 0,
            mdcr_el3: 0,
            brbe: Brbe {
                records_src: [BRBSRC_EL1_Type(0); 64],
                records_tgt: [BRBTGT_EL1_Type(0); 64],
                records_inf: [BRBINF_EL1_Type {
                    ccu: false,
                    cc: 0,
                    type_: 0,
                    el: 0,
                    mpred: false,
                    valid: 0,
                }; 64],
            },
            pmu: Pmu {
                pmcr_el0: 0,
                pmcntenset_el0: 0,
                pmcntenclr_el0: 0,
                pmccntr_el0: 0,
                counters: vec![],
                event_types: vec![],
            },
            pstate: pstate::PState::default(),
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum Feature {
    AA64,
    AA64EL0,
    AA64EL1,
    AA64EL2,
    AA64EL3,

    PAuth,

    AA32,
    AA32EL0,
    AA32EL1,
    AA32EL2,
    AA32EL3,

    SECURE,

    SEL2,
    VHE,

    E2H0,

    BRBE,
    BRBEv1p1,

    RME,

    PMUv3,

    #[allow(non_camel_case_types)]
    PMUv3_ICNTR,

    SPE,
    CMPBR,
}

impl Cpu {
    pub const NUM_PMU_COUNTERS: usize = 0;
    pub const PMU_EVENT_INST_RETIRED: u16 = 0x08;
    pub const PMU_EVENT_BRB_FILTRATE: u16 = 0x811F;

    pub const fn is_feature_implemented(feat: Feature) -> bool {
        match feat {
            Feature::AA64 => true,
            Feature::AA64EL0 => false,
            Feature::AA64EL1 => false,
            Feature::AA64EL2 => false,
            Feature::AA64EL3 => false,

            Feature::PAuth => false,
            Feature::SECURE => false,

            Feature::AA32 => false,
            Feature::AA32EL0 => false,
            Feature::AA32EL1 => false,
            Feature::AA32EL2 => false,
            Feature::AA32EL3 => false,

            Feature::SEL2 => false,
            Feature::VHE => false,
            Feature::E2H0 => false,

            Feature::BRBE => false,
            Feature::BRBEv1p1 => false,
            Feature::RME => false,

            Feature::PMUv3 => false,
            Feature::PMUv3_ICNTR => false,

            Feature::SPE => false,
            Feature::CMPBR => true,
        }
    }

    pub fn _get_op(&self, n: usize, cond: bool) -> u64 {
        if cond {
            self.x[n]
        } else {
            self.x[n] as u32 as u64
        }
    }

    pub fn _set_op(&mut self, n: usize, v: u64, cond: bool) {
        if cond {
            self.x[n] = v;
        } else {
            self.x[n] = to_uint::<32>(v);
        }
    }

    pub fn _get_reg_or_sp(&self, n: usize, cond: bool) -> u64 {
        if n == REG_SP {
            self.sp
        } else {
            self._get_op(n, cond)
        }
    }

    pub fn _set_reg_or_sp(&mut self, n: usize, value: u64, cond: bool) {
        if n == REG_SP {
            self.sp = value;
        } else {
            self._set_op(n, value, cond);
        }
    }

    /// A function for purely debug/testing purposes to arbitrarily set the value
    /// of a X register to a provided v value
    pub fn __dbg_set_x(&mut self, n: usize, v: u64) {
        self.x[n] = v;
    }

    /// A function for purely debug/testing purposes to arbitrarily fetch the value
    /// of a X register
    pub fn __dbg_get_x(&self, n: usize) -> u64 {
        self.x[n]
    }

    /// A function for purely debug/testing purposes to get a mutable reference
    /// to the CPU's PSTATE
    pub fn __dbg_get_pstate(&mut self) -> &mut pstate::PState {
        &mut self.pstate
    }

    /// **ADR**: Form PC-relative address
    ///
    /// This instruction adds an immediate value to the PC value to form a PC-relative address, and writes the result to the
    /// destination register.
    pub fn adr(&mut self, immlo: u8, immhi: u32, d: usize) {
        let immlo = to_uint::<2>(immlo as u64);
        let immhi = to_uint::<19>(immhi as u64);

        let imm = (immhi << 2) | immlo;
        self.x[d] = self.pc + imm;
    }

    /// **ADRP**: Form PC-relative address to 4KB page
    ///
    /// This instruction adds an immediate value that is shifted left by 12 bits, to the PC value to form a PC-relative address, with
    /// the bottom 12 bits masked out, and writes the result to the destination register.
    pub fn adrp(&mut self, immlo: u8, immhi: u32, d: usize) {
        let immlo = to_uint::<2>(immlo as u64);
        let immhi = to_uint::<19>(immhi as u64);

        let imm = ((immhi << 2) | immlo) << 12;
        let base = self.pc & (!0u64 << 12);

        self.x[d] = base + imm;
    }

    /// **AT**: Address translate
    ///
    /// For more information, see op0 == 0b01, cache maintenance, TLB maintenance, address translation, prediction restriction,
    /// BRBE, Trace Extension, Guarded Control Stack, and FEAT_RME_GPC3 instructions.
    #[allow(unused_variables)]
    pub fn at(&mut self, op1: u8, CRm: u8, op2: u8, t: usize) {
        todo!("AT is an alias of SYS which is yet to be implemented");
    }

    /// **AUTDA, AUTDZA**: Authenticate data address, using key A
    ///
    /// This instruction authenticates a data address, using a modifier and key A.
    ///
    /// The address is in the general-purpose register that is specified by &gt;Xd>.
    ///
    /// The modifier is:
    /// - In the general-purpose register or stack pointer that is specified by &gt;Xn|SP>, for AUTDA.
    /// - The value zero, for AUTDZA.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    #[allow(unused_variables)]
    pub fn autda_autdza(&mut self, Z: bool, n: usize, d: usize) {
        todo!("Not yet implemented");
    }

    /// **AUTDB, AUTDZB**: Authenticate data address, using key B
    ///
    /// This instruction authenticates a data address, using a modifier and key B.
    ///
    /// The address is in the general-purpose register that is specified by &gt;Xd>.
    ///
    /// The modifier is:
    /// - In the general-purpose register or stack pointer that is specified by &gt;Xn|SP> for AUTDB.
    /// - The value zero, for AUTDZB.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    #[allow(unused_variables)]
    pub fn autdb_autdzb(&mut self, Z: bool, n: usize, d: usize) {
        todo!("Not yet implemented");
    }

    /// AUTIA, AUTIA1716, AUTIASP, AUTIAZ, AUTIZA**: Authenticate instruction address, using key A
    ///
    /// This instruction authenticates an instruction address, using a modifier and key A.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    ///
    /// The address is:
    /// - In the general-purpose register that is specified by &gt;Xd> for AUTIA and AUTIZA.
    /// - In X17, for AUTIA1716.
    /// - In X30, for AUTIASP and AUTIAZ.
    ///
    /// The modifier is:
    /// - In the general-purpose register or stack pointer that is specified by &gt;Xn|SP> for AUTIA.
    /// - The value zero, for AUTIZA and AUTIAZ.
    /// - In X16, for AUTIA1716.
    /// - In SP, for AUTIASP.
    ///
    /// If FEAT_PAuth_LR is implemented and PSTATE.PACM is 1, then AUTIA1716 and AUTIASP include a second modifier
    /// that is:
    /// - In X15, for AUTIA1716.
    /// - In X16, for AUTIASP.
    ///
    /// It has encodings from 2 classes: Integer and System
    #[allow(unused_variables)]
    pub fn autia_autia1716_autiasp_autiaz_autiza(&mut self, Z: bool, n: usize, d: usize) {
        todo!("Not yet implemented");
    }

    /// **AUTIA171615**: Authenticate instruction address, using key A
    ///
    /// This instruction authenticates an instruction address, using two modifiers and key A. The address is in X17. The 64-bit
    /// value of modifier1 is the value in X16. The 64-bit value of modifier2 is the value in X15.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    pub fn autia171615(&mut self) {
        todo!("Not yet implemented");
    }

    /// **AUTIASPPC**: Authenticate return address using key A, using an immediate offset
    ///
    /// This instruction authenticates an instruction address, using two modifiers and key A.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    ///
    /// The address is in X30.
    ///
    /// The first modifier is in SP.
    ///
    /// The second modifier is the address of a program label.
    #[allow(unused_variables)]
    pub fn autiasppc(&mut self, imm16: u16) {
        todo!("Not yet implemented");
    }

    /// **AUTIASPPCR**: Authenticate return address using key A, using a register
    /// This instruction authenticates an instruction address, using two modifiers and key A.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    ///
    /// The address is in X30.
    ///
    /// The first modifier is in SP.
    ///
    /// The second modifier is in the general-purpose register that is specified by &gt;Xn>.
    #[allow(unused_variables)]
    pub fn autiasppcr(&mut self, n: usize) {
        todo!("Not yet implemented");
    }

    /// **AUTIB, AUTIB1716, AUTIBSP, AUTIBZ, AUTIZB**: Authenticate instruction address, using key B
    ///
    /// This instruction authenticates an instruction address, using a modifier and key B.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    ///
    /// The address is:
    /// - In the general-purpose register that is specified by &gt;Xd&lt; for AUTIB and AUTIZB.
    /// - In X17, for AUTIB1716.
    /// - In X30, for AUTIBSP and AUTIBZ.
    ///
    /// The modifier is:
    /// - In the general-purpose register or stack pointer that is specified by &gt;Xn|SP&lt; for AUTIB.
    /// - The value zero, for AUTIZB and AUTIBZ.
    /// - In X16, for AUTIB1716.
    /// - In SP, for AUTIBSP.
    ///
    /// If FEAT_PAuth_LR is implemented and PSTATE.PACM is 1, then AUTIB1716 and AUTIBSP include a second modifier
    /// that is:
    /// - In X15, for AUTIB1716.
    /// - In X16, for AUTIBSP.
    ///
    /// It has encodings from 2 classes: Integer and System
    #[allow(unused_variables)]
    pub fn autib_autib1716_autibsp_autibz_autizb(&mut self, Z: bool, n: usize, d: usize) {
        todo!("Not yet implemented");
    }

    /// **AUTIB171615**: Authenticate instruction address, using key B
    ///
    /// This instruction authenticates an instruction address, using two modifiers and key B. The address is in X17. The 64-bit
    /// value of modifier1 is the value in X16. The 64-bit value of modifier2 is the value in X15.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    pub fn autib171615(&mut self) {
        todo!("Not yet implemented");
    }

    /// **AUTIBSPPC**: Authenticate return address using key B, using an immediate offset
    ///
    /// This instruction authenticates an instruction address, using two modifiers and key B.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    ///
    /// The address is in X30.
    ///
    /// The first modifier is in SP.
    ///
    /// The second modifier is the address of a program label.
    #[allow(unused_variables)]
    pub fn autibsppc(&mut self, imm16: u16) {
        todo!("Not yet implemented");
    }

    /// **AUTIBSPPCR**: Authenticate return address using key B, using a register
    ///
    /// This instruction authenticates an instruction address, using two modifiers and key B.
    ///
    /// If the authentication passes, the upper bits of the address are restored to enable subsequent use of the address. For
    /// information on behavior if the authentication fails, see Faulting on pointer authentication.
    ///
    /// The address is in X30.
    ///
    /// The first modifier is in SP.
    ///
    /// The second modifier is in the general-purpose register that is specified by &gt;Xn>.
    #[allow(unused_variables)]
    pub fn autisppcr(&mut self, n: usize) {
        todo!("Not yet implemented");
    }

    /// **AXFLAG**: Convert floating-point condition flags from Arm to external format
    ///
    /// This instruction converts the state of the PSTATE.{N,Z,C,V} flags from a form representing the result of an Arm
    /// floating-point scalar compare instruction to an alternative representation required by some software.
    pub fn axflag(&mut self) {
        let z = self.pstate.Z || self.pstate.V;
        let c = self.pstate.C && !self.pstate.V;

        self.pstate.N = false;
        self.pstate.Z = z;
        self.pstate.C = c;
        self.pstate.V = false;
    }

    /// **BL**: Branch with link
    ///
    /// This instruction branches to a PC-relative offset, setting register X30 to PC+4. This instruction provides a hint that this is a
    /// subroutine call.
    #[allow(unused_variables)]
    pub fn bl(&mut self, imm26: u32) {
        todo!()
    }

    /// **BLR**: Branch with link to register
    ///
    /// This instruction calls a subroutine at an address in a register, setting register X30 to PC+4. This instruction provides a hint
    /// that this is a subroutine call.
    #[allow(unused_variables)]
    pub fn blr(&mut self, n: usize) {
        todo!()
    }

    /// **BLRAA, BLRAAZ, BLRAB, BLRABZ**: Branch with link to register, with pointer authentication
    ///
    /// This instruction authenticates the address in the general-purpose register that is specified by &lt;Xn&gt;, using a modifier and the
    /// specified key, and calls a subroutine at the authenticated address, setting register X30 to PC+4.
    ///
    /// The modifier is:
    /// - In the general-purpose register or stack pointer that is specified by <Xm|SP>, for BLRAA and BLRAB.
    /// - The value zero, for BLRAAZ and BLRABZ.
    ///
    /// Key A is used for BLRAA and BLRAAZ. Key B is used for BLRAB and BLRABZ.
    ///
    /// If the authentication passes, the PE continues execution at the target of the branch. For information on behavior if the
    /// authentication fails, see Faulting on pointer authentication.
    ///
    /// The authenticated address is not written back to the general-purpose register.
    ///
    /// This instruction provides a hint that this is a subroutine call.
    #[allow(unused_variables)]
    pub fn blraa_blraaz_blrab_blrabz(&mut self, Z: bool, M: bool, n: u8, m: u8) {
        todo!()
    }

    /// **BR**: Branch to register
    ///
    /// This instruction branches unconditionally to an address in a register. This instruction provides a hint that this is not a
    /// subroutine call or return.
    #[allow(unused_variables)]
    pub fn br(&mut self, n: usize) {
        todo!()
    }

    /// **BRAA, BRAAZ, BRAB, BRABZ**: Branch to register, with pointer authentication
    ///
    /// This instruction authenticates the address in the general-purpose register that is specified by &lt;Xn&gt;, using a modifier and the
    /// specified key, and branches to the authenticated address.
    ///
    /// The modifier is:
    /// - In the general-purpose register or stack pointer that is specified by <Xm|SP>, for BRAA and BRAB.
    /// - The value zero, for BRAAZ and BRABZ.
    ///
    /// Key A is used for BRAA and BRAAZ. Key B is used for BRAB and BRABZ.
    ///
    /// If the authentication passes, the PE continues execution at the target of the branch. For information on behavior if the
    /// authentication fails, see Faulting on pointer authentication.
    ///
    /// The authenticated address is not written back to the general-purpose register.
    ///
    /// This instruction provides a hint that this is not a subroutine call or return.
    #[allow(unused_variables)]
    pub fn braa_braaz_brab_brabz(&mut self, Z: bool, M: bool, n: u8, m: u8) {
        todo!()
    }

    /// **BRB**: Branch record buffer
    ///
    /// For more information, see op0 == 0b01, cache maintenance, TLB maintenance, address translation, prediction restriction,
    /// BRBE, Trace Extension, Guarded Control Stack, and FEAT_RME_GPC3 instructions.
    #[allow(unused_variables)]
    pub fn brb(&mut self, op2: u8, t: u8) {
        todo!()
    }

    /// **BRK**: Breakpoint instruction
    ///
    /// This instruction generates a Breakpoint Instruction exception. The PE records the exception in ESR_ELx, using the EC
    /// value 0x3C, and captures the value of the immediate argument in ESR_ELx.ISS.
    ///
    /// Within a guarded memory region, while PSTATE.BTYPE != 0b00, a BRK instruction will not generate a Branch Target
    /// exception and will generate a Breakpoint Instruction exception as normal. For more information, see PSTATE.BTYPE.
    #[allow(unused_variables)]
    pub fn brk(&mut self, imm16: u16) {
        todo!()
    }

    /// **BTI**: Branch target identification
    ///
    /// This instruction is used to guard against the execution of instructions that are not the intended target of a branch.
    ///
    /// Outside of a guarded memory region, a BTI instruction executes as a NOP. Within a guarded memory region, while
    /// PSTATE.BTYPE != 0b00, a BTI instruction compatible with the current value of PSTATE.BTYPE will not generate a
    /// Branch Target Exception and will allow execution of subsequent instructions within the memory region. For more
    /// information, see PSTATE.BTYPE.
    ///
    /// The operand &lt;targets&gt; passed to a BTI instruction determines the values of PSTATE.BTYPE that the BTI instruction is
    /// compatible with.
    #[allow(unused_variables)]
    pub fn bti(&mut self, op2: u8) {
        todo!()
    }

    /// **CAS, CASA, CASAL, CASL**: Compare and swap word or doubleword in memory
    ///
    /// This instruction reads a 32-bit word or 64-bit doubleword from memory, and compares it against the value held in a first
    /// register. If the comparison is equal, the value in a second register is written to memory. If the comparison is not equal, the
    /// architecture permits writing the value read from the location to memory. If the write is performed, the read and write occur
    /// atomically such that no other modification of the memory location can take place between the read and write.
    /// - If the destination register is not one of WZR or XZR, CASA and CASAL load from memory with acquire semantics.
    /// - CASL and CASAL store to memory with release semantics.
    /// - CAS has neither acquire nor release semantics.
    ///
    /// The architecture permits that the data read clears any exclusive monitors associated with that location, even if the compare
    /// subsequently fails.
    ///
    /// If the instruction generates a synchronous Data Abort, the register which is compared and loaded, that is &lt;Ws&gt;, or &lt;Xs&gt;, is
    /// restored to the value held in the register before the instruction was executed.
    ///
    /// For a CAS or CASA instruction, when &lt;Ws&gt; or &lt;Xs&gt; specifies the same register as &lt;Wt&gt; or &lt;Xt&gt;, this signals to the memory
    /// system that an additional subsequent CAS, CASA, CASAL, or CASL access to the specified location is expected to occur in the
    /// near future in the absence of taking an exception or exceptional software conditions. The memory system can respond by
    /// taking actions that are expected to enable the subsequent CAS, CASA, CASAL, or CASL access to succeed when it does occur.
    ///
    /// A code sequence starting with a CAS or CASA instruction for which &lt;Ws&gt; or &lt;Xs&gt; specifies the same register as &lt;Wt&gt; or
    /// &lt;Xt&gt;, and ending with a subsequent CAS, CASA, CASAL, or CASL to the same location, exhibits the following properties for
    /// best performance when the location may be accessed concurrently, on one or more other PEs:
    /// - The sequence does not contain any direct System register writes, address translation instructions, cache or TLB
    /// maintenance operations, exception producing instructions, exception returns, or ISB barriers.
    /// - The execution of the sequence includes 32 or fewer instructions.
    /// - The value provided in &lt;Ws&gt; or &lt;Xs&gt; of the first CAS or CASA is a value likely to result in the comparison failing. A
    /// failing comparison result may lead to better performance due to the hardware not performing a write to memory.
    ///
    /// NOTE:
    /// For a CAS or CASA instruction, when &lt;Ws&gt; or &lt;Xs&gt; specifies the same register as &lt;Wt&gt; or &lt;Xt&gt;, the value in memory is not
    /// modified, because the CAS or CASA either fails its compare or writes the same value back to memory.
    /// For performance reasons, Arm recommends that software not use a CAS or CASA instruction when &lt;Ws&gt; or &lt;Xs&gt; specifies the
    /// same register as &lt;Wt&gt; or &lt;Xt&gt; in a routine that includes a subsequent CAS, CASA, CASAL, or CASL for any of the following
    /// purposes:
    /// - To load the value that will be compared to conditionally determine whether or not to execute the subsequent CAS,
    /// CASA, CASAL, or CASL.
    /// - As a form of prefetch executed between the instruction loading the value that will be compared and the subsequent
    /// compare instruction that conditionally determines whether or not to execute the subsequent CAS, CASA, CASAL, or
    /// CASL.
    ///
    /// This guidance does not apply when &lt;Ws&gt; or &lt;Xs&gt; specifies the same register as &lt;Wt&gt; or &lt;Xt&gt; for a CAS or CASA instruction
    /// and the compare between the CAS or CASA and the subsequent CAS, CASA, CASAL, or CASL is intended to detect an
    /// exceptional software condition such that the result of the compare does not prevent the subsequent CAS, CASA, CASAL,
    /// or CASL from executing under normal operating conditions.
    #[allow(unused_variables)]
    pub fn cas_casa_casal_casl(
        &mut self,
        size: u8,
        A: bool,
        s: usize,
        R: bool,
        n: usize,
        t: usize,
    ) {
        todo!()
    }

    /// **CASB, CASAB, CASALB, CASLB**: Compare and swap byte in memory
    ///
    /// This instruction reads an 8-bit byte from memory, and compares it against the value held in a first register. If the comparison
    /// is equal, the value in a second register is written to memory. If the comparison is not equal, the architecture permits writing
    /// the value read from the location to memory. If the write is performed, the read and write occur atomically such that no other
    /// modification of the memory location can take place between the read and write.
    /// - If the destination register is not WZR, CASAB and CASALB load from memory with acquire semantics.
    /// - CASLB and CASALB store to memory with release semantics.
    /// - CASB has neither acquire nor release semantics.
    ///
    /// The architecture permits that the data read clears any exclusive monitors associated with that location, even if the compare
    /// subsequently fails.
    ///
    /// If the instruction generates a synchronous Data Abort, the register which is compared and loaded, that is &lt;Ws&gt;, is restored to
    /// the values held in the register before the instruction was executed.
    ///
    /// For a CASB or CASAB instruction, when &lt;Ws&gt; specifies the same register as &lt;Wt&gt;, this signals to the memory system that an
    /// additional subsequent CASB, CASAB, CASALB, or CASLB access to the specified location is expected to occur in the near
    /// future in the absence of taking an exception or exceptional software conditions. The memory system can respond by taking
    /// actions that are expected to enable the subsequent CASB, CASAB, CASALB, or CASLB access to succeed when it does occur.
    ///
    /// A code sequence starting with a CASB or CASAB instruction for which &lt;Ws&gt; specifies the same register as &lt;Wt&gt;, and ending
    /// with a subsequent CASB, CASAB, CASALB, or CASLB to the same location, exhibits the following properties for best
    /// performance when the location may be accessed concurrently, on one or more other PEs:
    /// - The sequence does not contain any direct System register writes, address translation instructions, cache or TLB
    /// maintenance operations, exception producing instructions, exception returns, or ISB barriers.
    /// - The execution of the sequence includes 32 or fewer instructions.
    /// - The value provided in &lt;Ws&gt; of the first CASB or CASAB is a value likely to result in the comparison failing. A failing
    /// comparison result may lead to better performance due to the hardware not performing a write to memory.
    ///
    /// NOTE:
    /// For a CASB or CASAB instruction, when &lt;Ws&gt; specifies the same register as &lt;Wt&gt;, the value in memory is not modified,
    /// because the CASB or CASAB either fails its compare or writes the same value back to memory.
    /// For performance reasons, Arm recommends that software not use a CASB or CASAB instruction when &lt;Ws&gt; specifies the same
    /// register as &lt;Wt&gt; in a routine that includes a subsequent CASB, CASAB, CASALB, or CASLB for any of the following purposes:
    /// - To load the value that will be compared to conditionally determine whether or not to execute the subsequent CASB,
    /// CASAB, CASALB, or CASLB.
    /// - As a form of prefetch executed between the instruction loading the value that will be compared and the subsequent
    /// compare instruction that conditionally determines whether or not to execute the subsequent CASB, CASAB, CASALB,
    /// or CASLB.
    ///
    /// This guidance does not apply when &lt;Ws&gt; specifies the same register as &lt;Wt&gt; for a CASB or CASAB instruction and the
    /// compare between the CASB or CASAB and the subsequent CASB, CASAB, CASALB, or CASLB is intended to detect an
    /// exceptional software condition such that the result of the compare does not prevent the subsequent CASB, CASAB, CASALB,
    /// or CASLB from executing under normal operating conditions.
    #[allow(unused_variables)]
    pub fn casb_casab_casalb_caslb(&mut self, A: bool, s: usize, R: bool, n: usize, t: usize) {
        todo!()
    }

    /// **CASH, CASAH, CASALH, CASLH**: Compare and swap halfword in memory
    ///
    /// This instruction reads a 16-bit halfword from memory, and compares it against the value held in a first register. If the
    /// comparison is equal, the value in a second register is written to memory. If the comparison is not equal, the architecture
    /// permits writing the value read from the location to memory. If the write is performed, the read and write occur atomically
    /// such that no other modification of the memory location can take place between the read and write.
    /// - If the destination register is not WZR, CASAH and CASALH load from memory with acquire semantics.
    /// - CASLH and CASALH store to memory with release semantics.
    /// - CASH has neither acquire nor release semantics.
    ///
    /// The architecture permits that the data read clears any exclusive monitors associated with that location, even if the compare
    /// subsequently fails.
    ///
    /// If the instruction generates a synchronous Data Abort, the register which is compared and loaded, that is &lt;Ws&gt;, is restored to
    /// the values held in the register before the instruction was executed.
    ///
    /// For a CASH or CASAH instruction, when &lt;Ws&gt; specifies the same register as &lt;Wt&gt;, this signals to the memory system that an
    /// additional subsequent CASH, CASAH, CASALH, or CASLH access to the specified location is expected to occur in the near
    /// future in the absence of taking an exception or exceptional software conditions. The memory system can respond by taking
    /// actions that are expected to enable the subsequent CASH, CASAH, CASALH, or CASLH access to succeed when it does occur.
    ///
    /// A code sequence starting with a CASH or CASAH instruction for which &lt;Ws&gt; specifies the same register as &lt;Wt&gt;, and ending
    /// with a subsequent CASH, CASAH, CASALH, or CASLH to the same location, exhibits the following properties for best
    /// performance when the location may be accessed concurrently, on one or more other PEs:
    /// - The sequence does not contain any direct System register writes, address translation instructions, cache or TLB
    /// maintenance operations, exception producing instructions, exception returns, or ISB barriers.
    /// - The execution of the sequence includes 32 or fewer instructions.
    /// - The value provided in &lt;Ws&gt; of the first CASH or CASAH is a value likely to result in the comparison failing. A failing
    /// comparison result may lead to better performance due to the hardware not performing a write to memory.
    ///
    /// NOTE:
    /// For a CASH or CASAH instruction, when &lt;Ws&gt; specifies the same register as &lt;Wt&gt;, the value in memory is not modified,
    /// because the CASH or CASAH either fails its compare or writes the same value back to memory.
    /// For performance reasons, Arm recommends that software not use a CASH or CASAH instruction when &lt;Ws&gt; specifies the same
    /// register as &lt;Wt&gt; in a routine that includes a subsequent CASH, CASAH, CASALH, or CASLH for any of the following purposes:
    /// - To load the value that will be compared to conditionally determine whether or not to execute the subsequent CASH,
    /// CASAH, CASALH, or CASLH.
    /// - As a form of prefetch executed between the instruction loading the value that will be compared and the subsequent
    /// compare instruction that conditionally determines whether or not to execute the subsequent CASH, CASAH, CASALH,
    /// or CASLH.
    ///
    /// This guidance does not apply when &lt;Ws&gt; specifies the same register as &lt;Wt&gt; for a CASH or CASAH instruction and the
    /// compare between the CASH or CASAH and the subsequent CASH, CASAH, CASALH, or CASLH is intended to detect an
    /// exceptional software condition such that the result of the compare does not prevent the subsequent CASH, CASAH, CASALH,
    /// or CASLH from executing under normal operating conditions.
    #[allow(unused_variables)]
    pub fn cash_casah_casalh_caslh(&mut self, A: bool, s: usize, R: bool, n: usize, t: usize) {
        todo!()
    }

    // TODO: CASP, CASPA, CASPAL, CASPL
    // TODO: CASPT, CASPAT, CASPALT, CASPLT
    // TODO: CAST, CASAT, CASALT, CASLT
}
