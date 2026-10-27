#![allow(dead_code, non_snake_case)]

use crate::{
    cpu::{
        brbe::{BRBINF_EL1_Type, BRBSRC_EL1_Type, BRBTGT_EL1_Type, Brbe},
        common::*,
        decode::Instruction,
        pmu::Pmu,
        regs::{MPAMVPMnEL2Type, S1PIRType, S1PORType},
    },
    memory::Memory,
};

mod aarch32;
mod aarch64;
mod accessors;
mod brbe;
pub mod common;
mod constraint;
pub mod decode;
mod exeception;
mod fault;
mod instructions;
mod memory;
// mod memory_acc;
mod pmu;
mod pstate;
mod regs;
mod shared;

const LOWER_32: u64 = 0xFFFF_FFFF_0000_0000;
const REG_SP: Register = 0b11111;
const REG_ZR: Register = 0b11111;

pub type Register = u8;

pub type Signal = bool;
pub const HIGH: Signal = true;
pub const LOW: Signal = false;

#[derive(Debug)]
pub enum CpuError {
    InstructionReadError,
}

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

#[derive(Debug)]
pub enum ShiftType {
    /// Logical Shift Left
    LSL = 0b00,
    /// Logical Shift Right
    LSR = 0b01,
    /// Arithmetic Shift Right
    ASR = 0b10,
    /// Rotate Right
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

pub enum DecodeType {
    NOP,
    UNDEF,
    OK,
}

#[derive(Debug)]
pub enum EndOfInstructionReason {
    Retired,
    Exception,
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
    _pc: u64,

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

    sctlr_el1: u64,
    sctlr_el2: u64,
    sctlr_el3: u64,

    sctlr2_el1: u64,
    sctlr2_el2: u64,
    sctlr2_el3: u64,

    edscr: u32,
    edscr2: u32,

    hcr_el2: u64,

    brbfcr_el0: u64,
    brbfcr_el1: u64,

    brbcr_el1: u64,
    brbcr_el2: u64,

    mdcr_el2: u64,
    mdcr_el3: u64,

    mdscr_el1: u64,

    mair_el1: u64,
    mair_el2: u64,
    mair_el3: u64,

    mpamidr_el1: u64,
    mpam0_el1: u64,
    mpam1_el1: u64,
    mpam2_el2: u64,
    mpam3_el3: u64,
    mpamsm_el1: u64,
    mpamhcr_el2: u64,
    mpamvpmv_el2: u64,

    mpamvpmn_el2: [MPAMVPMnEL2Type; 8],

    pir_el1: S1PIRType,
    pir_el2: S1PIRType,
    pir_el3: S1PIRType,

    tcr2_el1: u64,
    tcr2_el2: u64,

    // NOTE: change to u128 when D128 feature is implemented
    ttbr0_el1: u64,
    ttbr0_el2: u64,
    ttbr0_el3: u64,

    // NOTE: change to u128 when D128 feature is implemented
    ttbr1_el1: u64,
    ttbr1_el2: u64,

    vtcr_el2: u64,

    mecid_p0_el2: u64,
    mecid_a0_el2: u64,
    mecid_p1_el2: u64,
    mecid_a1_el2: u64,
    vmecid_p_el2: u64,
    mecid_rl_a_el3: u64,

    hcrx_el2: u64,

    por_el0: S1PORType,
    por_el1: S1PORType,
    por_el2: S1PORType,
    por_el3: S1PORType,

    gpccr_el3: u64,
    gpcbw_el3: u64,
    gptbr_el3: u64,

    hcr: u32,
    hcr2: u32,

    oslsr_el1: u64,
    osdlr_el1: u64,
    dbgosdlr: u32,
    dbgprcr: u32,
    dbgprcr_el1: u64,

    sder32_el3: u64,

    dbgwcr_el1: [u64; NUM_WATCHPOINTS_IMPLEMENTED],
    dbgbcr_el1: [u64; NUM_BREAKPOINTS_IMPLEMENTED],

    brbe: Brbe,
    pmu: Pmu,

    dbgen: Signal,
    niden: Signal,
    spiden: Signal,
    spniden: Signal,
    rlpiden: Signal,
    rtpiden: Signal,

    pstate: pstate::PState,

    btype_next: u8,
}

impl Default for Cpu {
    /// WARN: Use for testing only!!
    fn default() -> Self {
        Self {
            x: [0; 31],
            sp: 0,
            _pc: 0,
            v: [0; 32],
            fcpr: 0,
            fpsr: 0,
            tcr_el1: 0,
            tcr_el2: 0,
            tcr_el3: 0,
            scr: 0,
            scr_el3: 0,
            sctlr_el1: 0,
            sctlr_el2: 0,
            sctlr_el3: 0,
            sctlr2_el1: 0,
            sctlr2_el2: 0,
            sctlr2_el3: 0,
            edscr: 0,
            edscr2: 0,
            hcr_el2: 0,
            brbfcr_el0: 0,
            brbfcr_el1: 0,
            brbcr_el1: 0,
            brbcr_el2: 0,
            mdcr_el2: 0,
            mdcr_el3: 0,
            mdscr_el1: 0,
            mpamidr_el1: 0,
            mair_el1: 0,
            mair_el2: 0,
            mair_el3: 0,
            mpam0_el1: 0,
            mpam1_el1: 0,
            mpam2_el2: 0,
            mpam3_el3: 0,
            mpamsm_el1: 0,
            mpamhcr_el2: 0,
            mpamvpmv_el2: 0,
            mpamvpmn_el2: [0; 8],
            pir_el1: [0; 16],
            pir_el2: [0; 16],
            pir_el3: [0; 16],
            tcr2_el1: 0,
            tcr2_el2: 0,
            ttbr0_el1: 0,
            ttbr0_el2: 0,
            ttbr0_el3: 0,
            ttbr1_el1: 0,
            ttbr1_el2: 0,
            vtcr_el2: 0,
            mecid_p0_el2: 0,
            mecid_a0_el2: 0,
            mecid_p1_el2: 0,
            mecid_a1_el2: 0,
            vmecid_p_el2: 0,
            mecid_rl_a_el3: 0,
            hcrx_el2: 0,
            por_el0: [0; 16],
            por_el1: [0; 16],
            por_el2: [0; 16],
            por_el3: [0; 16],
            gpccr_el3: 0,
            gpcbw_el3: 0,
            gptbr_el3: 0,
            hcr: 0,
            hcr2: 0,
            oslsr_el1: 0,
            osdlr_el1: 0,
            dbgosdlr: 0,
            dbgprcr: 0,
            dbgprcr_el1: 0,
            sder32_el3: 0,
            dbgwcr_el1: [0; NUM_WATCHPOINTS_IMPLEMENTED],
            dbgbcr_el1: [0; NUM_BREAKPOINTS_IMPLEMENTED],
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
            dbgen: LOW,
            niden: LOW,
            spiden: LOW,
            spniden: LOW,
            rlpiden: LOW,
            rtpiden: LOW,

            btype_next: 0,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
pub enum Feature {
    AA32,
    AA32EL0,
    AA32EL1,
    AA32EL2,
    AA32EL3,
    AA64,
    AA64EL0,
    AA64EL1,
    AA64EL2,
    AA64EL3,
    ABLE,
    AIE,
    BRBE,
    BRBEv1p1,
    BTI,
    BWE2,
    CMPBR,
    CSSC,
    D128,
    Debugv8p9,
    DoubleFault2,
    DoubleLock,
    E2H0,
    ExS,
    FlagM,
    FlagM2,
    GCS,
    HAF,
    HAFT,
    HAFDBS,
    HBC,
    HCX,
    HPDS,
    IESB,
    LPA,
    LPA2,
    LS64WB,
    LSE2,
    MEC,
    MPAMv0p1,
    MPAMv1p1,
    MTE2,
    #[allow(non_camel_case_types)]
    MTE_CANONICAL_TAGS,
    #[allow(non_camel_case_types)]
    MTE_NO_ADDRESS_TAGS,
    PAuth,
    PAN,
    PMUv3,
    #[allow(non_camel_case_types)]
    PMUv3_ICNTR,
    RAS,
    RASv2,
    RME,
    #[allow(non_camel_case_types)]
    RME_GDI,
    #[allow(non_camel_case_types)]
    RME_GPC2,
    #[allow(non_camel_case_types)]
    RME_GPC3,
    S1PIE,
    S1POE,
    SCTLR2,
    Secure,
    SEL2,
    SME,
    SPE,
    TCR2,
    THE,
    VHE,
}

impl Cpu {
    pub const NUM_PMU_COUNTERS: usize = 0;
    pub const PMU_EVENT_INST_RETIRED: u16 = 0x08;
    pub const PMU_EVENT_BRB_FILTRATE: u16 = 0x811F;

    pub const fn is_feature_implemented(feat: Feature) -> bool {
        match feat {
            Feature::AA32 => false,
            Feature::AA32EL0 => false,
            Feature::AA32EL1 => false,
            Feature::AA32EL2 => false,
            Feature::AA32EL3 => false,
            Feature::AA64 => true,
            Feature::AA64EL0 => false,
            Feature::AA64EL1 => false,
            Feature::AA64EL2 => false,
            Feature::AA64EL3 => false,
            Feature::ABLE => false,
            Feature::AIE => false,
            Feature::BRBE => false,
            Feature::BRBEv1p1 => false,
            Feature::BTI => false,
            Feature::BWE2 => false,
            Feature::CMPBR => true,
            Feature::CSSC => true,
            Feature::D128 => false,
            Feature::Debugv8p9 => false,
            Feature::DoubleFault2 => false,
            Feature::DoubleLock => false,
            Feature::E2H0 => false,
            Feature::ExS => false,
            Feature::FlagM => false,
            Feature::FlagM2 => false,
            Feature::GCS => false,
            Feature::HAF => false,
            Feature::HAFT => false,
            Feature::HAFDBS => false,
            Feature::HBC => false,
            Feature::HCX => false,
            Feature::HPDS => false,
            Feature::IESB => false,
            Feature::LPA => false,
            Feature::LPA2 => false,
            Feature::LS64WB => false,
            Feature::LSE2 => false,
            Feature::MEC => false,
            Feature::MPAMv0p1 => false,
            Feature::MPAMv1p1 => false,
            Feature::MTE2 => false,
            Feature::MTE_CANONICAL_TAGS => false,
            Feature::MTE_NO_ADDRESS_TAGS => false,
            Feature::PAuth => false,
            Feature::PAN => false,
            Feature::PMUv3 => false,
            Feature::PMUv3_ICNTR => false,
            Feature::RAS => false,
            Feature::RASv2 => false,
            Feature::RME => false,
            Feature::RME_GDI => false,
            Feature::RME_GPC2 => false,
            Feature::RME_GPC3 => false,
            Feature::S1PIE => false,
            Feature::S1POE => false,
            Feature::SCTLR2 => false,
            Feature::Secure => false,
            Feature::SEL2 => false,
            Feature::SME => false,
            Feature::SPE => false,
            Feature::TCR2 => false,
            Feature::THE => false,
            Feature::VHE => false,
        }
    }

    pub fn _get_op(&self, n: Register, cond: bool) -> u64 {
        if cond {
            self.x[n as usize]
        } else {
            self.x[n as usize] as u32 as u64
        }
    }

    pub fn _set_op(&mut self, n: Register, v: u64, cond: bool) {
        if cond {
            self.x[n as usize] = v;
        } else {
            self.x[n as usize] = to_uint::<32>(v);
        }
    }

    pub fn _get_reg_or_sp(&self, n: Register, cond: bool) -> u64 {
        if n == REG_SP {
            self.sp
        } else {
            self._get_op(n, cond)
        }
    }

    pub fn _set_reg_or_sp(&mut self, n: Register, value: u64, cond: bool) {
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
    pub fn adr(&mut self, immlo: u8, immhi: u32, d: Register) {
        let immlo = to_uint::<2>(immlo as u64);
        let immhi = to_uint::<19>(immhi as u64);

        let imm = (immhi << 2) | immlo;
        self._set_op(d, self.get_PC64().wrapping_add(imm), true);
    }

    /// **ADRP**: Form PC-relative address to 4KB page
    ///
    /// This instruction adds an immediate value that is shifted left by 12 bits, to the PC value to form a PC-relative address, with
    /// the bottom 12 bits masked out, and writes the result to the destination register.
    pub fn adrp(&mut self, immlo: u8, immhi: u32, d: Register) {
        let immlo = to_uint::<2>(immlo as u64);
        let immhi = to_uint::<19>(immhi as u64);

        let imm = ((immhi << 2) | immlo) << 12;
        let base = self.get_PC64() & (!0u64 << 12);

        self._set_op(d, base.wrapping_add(imm), true);
    }

    /// **AXFLAG**: Convert floating-point condition flags from Arm to external format
    ///
    /// This instruction converts the state of the PSTATE.{N,Z,C,V} flags from a form representing the result of an Arm
    /// floating-point scalar compare instruction to an alternative representation required by some software.
    pub fn axflag(&mut self) {
        let z = self.pstate.Z() || self.pstate.V();
        let c = self.pstate.C() && !self.pstate.V();

        self.pstate.set_nzcv(&NZCV::init(false, z, c, false));
    }

    fn execute_instruction(&mut self, instr: Instruction) {
        match instr {
            Instruction::Abs { sf, rn, rd } => self.abs(sf, rn, rd),
            Instruction::Adc { sf, rm, rn, rd } => self.adc(sf, rm, rn, rd),
            Instruction::Adcs { sf, rm, rn, rd } => self.adcs(sf, rm, rn, rd),
            Instruction::AddEr {
                sf,
                rm,
                option,
                imm3,
                rn,
                rd,
            } => self.add_er(sf, rm, option, imm3, rn, rd),
            Instruction::AddImm {
                sf,
                sh,
                imm12,
                rn,
                rd,
            } => self.add_imm(sf, sh, imm12, rn, rd),
            Instruction::AddSr {
                sf,
                shift,
                rm,
                imm6,
                rn,
                rd,
            } => self.add_sr(sf, shift, rm, imm6, rn, rd),
            Instruction::BCond { imm19, cond } => self.b_cond(imm19, cond),
            Instruction::Movz { sf, rd, imm16, hw } => self.movz(sf, hw, imm16, rd),
            Instruction::SubEr {
                sf,
                rm,
                option,
                imm3,
                rn,
                rd,
            } => self.sub_er(sf, rm, option, imm3, rn, rd),
            Instruction::SubImm {
                sf,
                sh,
                imm12,
                rn,
                rd,
            } => self.sub_imm(sf, sh, imm12, rn, rd),
            Instruction::SubSr {
                sf,
                shift,
                rm,
                imm6,
                rn,
                rd,
            } => self.sub_sr(sf, shift, rm, imm6, rn, rd),
            Instruction::SubsEr {
                sf,
                rm,
                option,
                imm3,
                rn,
                rd,
            } => self.subs_er(sf, rm, option, imm3, rn, rd),
            Instruction::SubsImm {
                sf,
                sh,
                imm12,
                rn,
                rd,
            } => self.subs_imm(sf, sh, imm12, rn, rd),
            Instruction::SubsSr {
                sf,
                shift,
                rm,
                imm6,
                rn,
                rd,
            } => self.subs_sr(sf, shift, rm, imm6, rn, rd),
        }
    }

    pub fn step(&mut self, memory: &mut Memory) -> Result<(), CpuError> {
        let instruction_word = memory
            .read::<u32>(self.get_PC64() as usize)
            .ok_or(CpuError::InstructionReadError)?;

        let decoded = decode::decode(instruction_word);
        if let Some(instr) = decoded {
            println!("Executing instruction: {:?}", instr);
            self.execute_instruction(instr);
        }

        self._pc += 4;

        Ok(())
    }
}
