#![allow(dead_code, non_snake_case)]

use crate::cpu::{
    brbe::{BRBINF_EL1_Type, BRBSRC_EL1_Type, BRBTGT_EL1_Type, Brbe},
    pmu::Pmu,
    shared::decode_bit_masks,
};

mod brbe;
pub mod common;
mod pmu;
mod pstate;
mod regs;
mod shared;

const LOWER_32: u64 = 0xFFFF_FFFF_0000_0000;

pub enum RegisterSP {
    Register(usize),
    StackPointer,
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

impl Cpu {
    pub const FEAT_AA64: bool = true;
    pub const FEAT_AA64EL0: bool = true;
    pub const FEAT_AA64EL1: bool = true;
    pub const FEAT_AA64EL2: bool = true;
    pub const FEAT_AA64EL3: bool = true;

    #[allow(non_upper_case_globals)]
    pub const FEAT_PAuth: bool = false;

    pub const FEAT_AA32EL0: bool = false;
    pub const FEAT_AA32EL1: bool = false;
    pub const FEAT_AA32EL2: bool = false;
    pub const FEAT_AA32EL3: bool = false;

    pub const FEAT_SECURE: bool = false;

    pub const FEAT_SEL2: bool = false;
    pub const FEAT_VHE: bool = false;

    pub const FEAT_E2H0: bool = false;

    pub const FEAT_BRBE: bool = false;
    #[allow(non_upper_case_globals)]
    pub const FEAT_BRBEv1p1: bool = false;

    pub const FEAT_RME: bool = false;

    #[allow(non_upper_case_globals)]
    pub const FEAT_PMUv3: bool = false;
    #[allow(non_upper_case_globals)]
    pub const FEAT_PMUv3_ICNTR: bool = false;
    pub const NUM_PMU_COUNTERS: usize = 0;
    pub const PMU_EVENT_INST_RETIRED: u16 = 0x08;
    const PMU_EVENT_BRB_FILTRATE: u16 = 0x811F;

    pub const FEAT_SPE: bool = false;

    fn _get_op(&self, n: usize, cond: bool) -> u64 {
        if cond {
            self.x[n]
        } else {
            self.x[n] as u32 as u64
        }
    }

    fn _set_op(&mut self, n: usize, v: u64, cond: bool) {
        if cond {
            self.x[n] = v;
        } else {
            self.x[n] = to_uint::<32>(v);
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

    /// **ABS**: Absolute value
    ///
    /// This instruction calculates the absolute value of the signed integer value in the source register, and writes the result to the
    /// destination register. If the value in the source register is the most negative value, the result is the same as the value in the
    /// source register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, -5i64 as u64);
    /// cpu.abs(true, 0, 1);  // `ABS X1, X0`
    /// assert_eq!(cpu.__dbg_get_x(1), 5);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 9223372032559808513u64);
    /// cpu.abs(false, 0, 1);  // `ABS W1, W0`
    /// assert_eq!(cpu.__dbg_get_x(1), 1);
    /// ```
    pub fn abs(&mut self, sf: bool, n: usize, d: usize) {
        let operand1 = self._get_op(n, sf) as i64;

        let result = operand1.abs() as u64;
        self._set_op(d, result, sf);
    }

    /// **ADC**: Add with carry
    ///
    /// This instruction adds two register values and the Carry flag value, and writes the result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// cpu.adc(true, 1, 0, 2);  // `ADC X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 3);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// cpu.__dbg_get_pstate().__dbg_set_C(true);  // Set PSTATE.C = 1
    /// cpu.adc(true, 1, 0, 2);  // `ADC X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 4);
    /// ```
    pub fn adc(&mut self, sf: bool, m: usize, n: usize, d: usize) {
        let operand1 = self._get_op(n, sf);
        let operand2 = self._get_op(m, sf);

        let (result, _) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C)
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C)
        };

        self._set_op(d, result, sf);
    }

    /// **ADCS**: Add with carry, setting flags
    ///
    /// This instruction adds two register values and the Carry flag value, and writes the result to the destination register. It updates
    /// the condition flags based on the result.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// cpu.adcs(true, 1, 0, 2);  // `ADCS X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 3);
    /// assert!(!cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// cpu.__dbg_set_x(1, 1);
    /// cpu.adcs(true, 1, 0, 2);  // `ADCS X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 0);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn adcs(&mut self, sf: bool, m: usize, n: usize, d: usize) {
        let operand1 = self._get_op(n, sf);
        let operand2 = self._get_op(m, sf);

        let (result, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C)
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C)
        };

        self._set_op(d, result, sf);
        nzcv.set(&mut self.pstate);
    }

    /// **ADD (extended register)**: Add extended and scaled register
    ///
    /// This instruction adds a register value and a sign-extended or zero-extended register value, followed by an optional left shift
    /// amount, and writes the result to the destination register. The argument that is extended from the &lt;Rm&gt; register can be a byte,
    /// halfword, word, or doubleword.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// // `ADD X2, X0, X1, UXTB #3`
    /// cpu._add_er(true, 1, 0b000, 3, RegisterSP::Register(0), RegisterSP::Register(2));
    /// assert_eq!(cpu.__dbg_get_x(2), 17);
    /// ```
    ///
    /// We get 17 because 17 = 1 + (2 << 3)
    /// The ExtendType is not very consequential because we are dealing with such small values.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 500);
    /// // `ADD X2, X0, X1, UXTB #3`
    /// cpu._add_er(true, 1, 0b000, 3, RegisterSP::Register(0), RegisterSP::Register(2));
    /// assert_eq!(cpu.__dbg_get_x(2), 1953);
    /// ```
    ///
    /// We get 1953 because 1953 = 1 + (244 << 3), where 244 = 500 & 0xFF (due to UXTB extending)
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 500);
    /// // `ADD X2, X0, X1, UXTD #3`
    /// cpu._add_er(true, 1, 0b011, 3, RegisterSP::Register(0), RegisterSP::Register(2));
    /// assert_eq!(cpu.__dbg_get_x(2), 4001);
    /// ```
    ///
    /// Without the masking to byte-width, we get 4001 instead.
    pub fn _add_er(
        &mut self,
        sf: bool,
        m: usize,
        option: u8,
        imm3: u8,
        n: RegisterSP,
        d: RegisterSP,
    ) {
        let operand1 = if let RegisterSP::Register(rn) = n {
            self._get_op(rn, sf)
        } else {
            self.sp
        };

        let mval = self._get_op(m, option & 0b11 == 0b11);
        let extend_type = ExtendType::decode(option).unwrap();

        let (result, _) = if sf {
            let operand2 = shared::extend_reg::<64>(mval, extend_type, imm3);
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            let operand2 = shared::extend_reg::<32>(mval, extend_type, imm3);
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        if let RegisterSP::Register(rd) = d {
            self._set_op(rd, result, sf);
        } else {
            self.sp = result;
        }
    }

    /// **ADD (Immediate)**: Add immediate value
    ///
    /// This instruction adds a register value and an optionally-shifted immediate value, and writes the result to the destination
    /// register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// // `ADD X1, X0, #10 LSL #0`
    /// cpu._add_imm(true, false, 10, RegisterSP::Register(0), RegisterSP::Register(1));
    /// assert_eq!(cpu.__dbg_get_x(1), 11);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// // `ADD X1, X0, #10 LSL #12`
    /// cpu._add_imm(true, true, 10, RegisterSP::Register(0), RegisterSP::Register(1));
    /// assert_eq!(cpu.__dbg_get_x(1), 40961);
    /// ```
    pub fn _add_imm(&mut self, sf: bool, sh: bool, imm12: u16, n: RegisterSP, d: RegisterSP) {
        let imm12 = imm12 & 0b1111_1111_1111;
        let imm = if sh {
            (imm12 as u32) << 12
        } else {
            imm12 as u32
        };

        let operand1 = if let RegisterSP::Register(rn) = n {
            self._get_op(rn, sf)
        } else {
            self.sp
        };

        let (result, _) = if sf {
            shared::add_with_carry::<64>(operand1, imm as u64, false)
        } else {
            shared::add_with_carry::<32>(operand1, imm as u64, false)
        };

        if let RegisterSP::Register(rd) = d {
            self._set_op(rd, result, sf);
        } else {
            self.sp = result;
        }
    }

    /// **ADD (Shifted Register)**: Add optionally-shifted register
    ///
    /// This instruction adds a register value and an optionally-shifted register value, and writes the result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// // `ADD X2, X0, X1, LSL #3`
    /// cpu._add_sr(true, 0b00, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 17);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 36);
    /// // `ADD X2, X0, X1, LSR #3`
    /// cpu._add_sr(true, 0b01, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 5);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, -44i64 as u64);
    /// // `ADD X2, X0, X1, ASR #2`
    /// cpu._add_sr(true, 0b10, 1, 2, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), -10i64 as u64);
    /// ```
    pub fn _add_sr(&mut self, sf: bool, shift: u8, m: usize, imm6: u8, n: usize, d: usize) {
        let operand1 = self._get_op(n, sf);
        let mval = self._get_op(m, sf);

        let shift_type = ShiftType::decode(shift).unwrap();

        let operand2 = shared::shift_reg(mval, shift_type, imm6 as usize);

        let (result, _) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        self._set_op(d, result, sf);
    }

    fn addg(&mut self) {
        todo!()
    }

    fn addpt(&mut self) {
        todo!()
    }

    /// **ADDS (extended register)**: Add extended and scaled register, setting flags
    ///
    /// This instruction adds a register value and a sign-extended or zero-extended register value, followed by an optional left shift
    /// amount, and writes the result to the destination register. The argument that is extended from the &lt;Rm&gt; register can be a byte,
    /// halfword, word, or doubleword. It updates the condition flags based on the result.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, u64::MAX >> 2);
    /// // `ADDS X2, X0, X1, UXTB #3`
    /// cpu._adds_er(true, 1, 0b011, 0b010, RegisterSP::Register(0), RegisterSP::Register(2));
    /// assert_eq!(cpu.__dbg_get_x(2), 1);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn _adds_er(
        &mut self,
        sf: bool,
        m: usize,
        option: u8,
        imm3: u8,
        n: RegisterSP,
        d: RegisterSP,
    ) {
        let operand1 = if let RegisterSP::Register(rn) = n {
            self._get_op(rn, sf)
        } else {
            self.sp
        };

        let mval = self._get_op(m, option & 0b11 == 0b11);
        let extend_type = ExtendType::decode(option).unwrap();

        let (result, nzcv) = if sf {
            let operand2 = shared::extend_reg::<64>(mval, extend_type, imm3);
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            let operand2 = shared::extend_reg::<32>(mval, extend_type, imm3);
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        nzcv.set(&mut self.pstate);
        if let RegisterSP::Register(rd) = d {
            self._set_op(rd, result, sf);
        } else {
            self.sp = result;
        }
    }

    /// **ADDS (Immediate)**: Add immediate value, setting flags
    ///
    /// This instruction adds a register value and an optionally-shifted immediate value, and writes the result to the destination
    /// register. It updates the condition flags based on the result.
    ///
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// // `ADDS X1, X0, #10 LSL #0`
    /// cpu._adds_imm(true, false, 10, RegisterSP::Register(0), RegisterSP::Register(1));
    /// assert_eq!(cpu.__dbg_get_x(1), 9);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// // `ADDS X1, X0, #10 LSL #12`
    /// cpu._adds_imm(true, true, 10, RegisterSP::Register(0), RegisterSP::Register(1));
    /// assert_eq!(cpu.__dbg_get_x(1), 40959);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn _adds_imm(&mut self, sf: bool, sh: bool, imm12: u16, n: RegisterSP, d: RegisterSP) {
        let imm12 = imm12 & 0b1111_1111_1111;
        let imm = if sh {
            (imm12 as u32) << 12
        } else {
            imm12 as u32
        };

        let operand1 = if let RegisterSP::Register(rn) = n {
            self._get_op(rn, sf)
        } else {
            self.sp
        };

        let (result, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, imm as u64, false)
        } else {
            shared::add_with_carry::<32>(operand1, imm as u64, false)
        };

        nzcv.set(&mut self.pstate);
        if let RegisterSP::Register(rd) = d {
            self._set_op(rd, result, sf);
        } else {
            self.sp = result;
        }
    }

    /// **ADDS (Shifted Register)**: Add optionally-shifted register, setting flags
    ///
    /// This instruction adds a register value and an optionally-shifted register value, and writes the result to the destination register.
    /// It updates the condition flags based on the result.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 8);
    /// cpu.__dbg_set_x(1, u64::MAX >> 3);
    /// // `ADDS X2, X0, X1, LSL #3`
    /// cpu._adds_sr(true, 0b00, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 0);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// cpu.__dbg_set_x(1, 36);
    /// // `ADDS X2, X0, X1, LSR #3`
    /// cpu._adds_sr(true, 0b01, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 3);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MIN);
    /// cpu.__dbg_set_x(1, -44i64 as u64);
    /// // `ADDS X2, X0, X1, ASR #2`
    /// cpu._adds_sr(true, 0b10, 1, 2, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), -11i64 as u64);
    /// assert!(!cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// This does NOT overflow
    pub fn _adds_sr(&mut self, sf: bool, shift: u8, m: usize, imm6: u8, n: usize, d: usize) {
        let operand1 = self._get_op(n, sf);
        let mval = self._get_op(m, sf);

        let shift_type = ShiftType::decode(shift).unwrap();

        let operand2 = shared::shift_reg(mval, shift_type, imm6 as usize);

        let (res, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        self._set_op(d, res, sf);
        nzcv.set(&mut self.pstate);
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

    /// **AND (immediate)**: Bitwise AND (immediate)
    ///
    /// This instruction performs a bitwise AND between a register value and an immediate value, and writes the result to the
    /// destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0x1234_5678_9ABC_DEF0);
    /// // `AND X1, X0, #0xFF`
    /// cpu._and_imm(true, true, 0b000000, 0b000111, 0, RegisterSP::Register(1));
    /// assert_eq!(cpu.__dbg_get_x(1), 0xF0);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFFFF_FFFF_FFFF_FFFF);
    /// // `AND X1, X0, #0x00`
    /// cpu._and_imm(true, false, 0b000000, 0b110100, 0, RegisterSP::Register(1));
    /// assert_eq!(cpu.__dbg_get_x(1), 0x1F1F_1F1F_1F1F_1F1F);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 232131);
    /// // `AND X1, X0, #0x00`
    /// cpu._and_imm(true, false, 0b001101, 0b101011, 0, RegisterSP::Register(1));
    /// assert_eq!(cpu.__dbg_get_x(1), 2752);
    /// ```
    ///
    /// Special thanks to <https://gist.github.com/dinfuehr/51a01ac58c0b23e4de9aac313ed6a06a>
    pub fn _and_imm(&mut self, sf: bool, imm_n: bool, immr: u8, imms: u8, n: usize, d: RegisterSP) {
        let (imm, _) = if sf {
            decode_bit_masks::<64>(imm_n, imms, immr)
        } else {
            decode_bit_masks::<32>(imm_n, imms, immr)
        };

        let operand1 = self._get_op(n, sf);
        let operand2 = imm;

        println!("{} {}", operand1, operand2);
        let result = operand1 & operand2;

        if let RegisterSP::Register(rd) = d {
            self._set_op(rd, result, sf);
        } else {
            self.sp = result;
        }
    }

    /// **AND (Shifted Register)**: Bitwise AND (shifted register)
    ///
    /// This instruction performs a bitwise AND between a register value and an optionally-shifted register value, and writes the
    /// result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 213);
    /// cpu.__dbg_set_x(1, 82);
    /// cpu._and_sr(true, 0b00, 1, 0, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 80);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, RegisterSP, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 213);
    /// cpu.__dbg_set_x(1, 5);
    /// cpu._and_sr(true, 0b00, 1, 4, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 80);
    /// ```
    pub fn _and_sr(&mut self, sf: bool, shift: u8, m: usize, imm6: u8, n: usize, d: usize) {
        let operand1 = self._get_op(n, sf);
        let operand2 = shared::shift_reg(
            self._get_op(m, sf),
            ShiftType::decode(shift).unwrap(),
            imm6 as usize,
        );

        self._set_op(d, operand1 & operand2, sf);
    }

    /// **ANDS (Immediate)**: Bitwise AND (immediate), setting flags
    ///
    /// This instruction performs a bitwise AND between a register value and an immediate value, and writes the result to the
    /// destination register. It updates the condition flags based on the result.
    pub fn _ands_imm(
        &mut self,
        sf: bool,
        imm_n: bool,
        immr: u8,
        imms: u8,
        n: usize,
        d: RegisterSP,
    ) {
        let (imm, _) = if sf {
            decode_bit_masks::<64>(imm_n, imms, immr)
        } else {
            decode_bit_masks::<32>(imm_n, imms, immr)
        };

        let operand1 = self._get_op(n, sf);
        let operand2 = imm;

        println!("{} {}", operand1, operand2);
        let result = operand1 & operand2;

        if let RegisterSP::Register(rd) = d {
            self._set_op(rd, result, sf);
        } else {
            self.sp = result;
        }

        self.pstate.N = (result >> ((32 << sf as u64) - 1)) == 1;
        self.pstate.Z = result == 0;
        self.pstate.C = false;
        self.pstate.V = false;
    }

    /// **ANDS: (shifted register)**: Bitwise AND (shifted register), setting flags
    ///
    /// This instruction performs a bitwise AND between a register value and an optionally-shifted register value, and writes the
    /// result to the destination register. It updates the condition flags based on the result.
    pub fn _ands_sr(&mut self, sf: bool, shift: u8, m: usize, imm6: u8, n: usize, d: usize) {
        let operand1 = self._get_op(n, sf);
        let operand2 = shared::shift_reg(
            self._get_op(m, sf),
            ShiftType::decode(shift).unwrap(),
            imm6 as usize,
        );

        let result = operand1 & operand2;
        self._set_op(d, result, sf);

        self.pstate.N = (result >> ((32 << sf as u64) - 1)) == 1;
        self.pstate.Z = result == 0;
        self.pstate.C = false;
        self.pstate.V = false;
    }

    /// **ASR (register)**: Arithmetic shift right (register)
    ///
    /// This instruction shifts a register value right by a variable number of bits, shifting in copies of its sign bit, and writes the
    /// result to the destination register. The value of the second source register modulo the register size in bits gives the number of
    /// bits by which the first source register is right-shifted.
    pub fn _asr_reg(&mut self, sf: bool, m: usize, n: usize, d: usize) {
        self.asrv(sf, m, n, d);
    }

    /// **ASR (immediate)**: Arithmetic shift right (immediate)
    ///
    /// This instruction shifts a register value right by an immediate number of bits, shifting in copies of the sign bit in the upper
    /// bits and zeros in the lower bits, and writes the result to the destination register.
    #[allow(unused_variables)]
    pub fn _asr_imm(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: usize, d: usize) {
        todo!("ASR (immediate) is an alias of SBFM which is yet to be implemented")
    }

    /// **ASRV**: Arithmetic shift right variable
    ///
    /// This instruction shifts a register value right by a variable number of bits, shifting in copies of its sign bit, and writes the
    /// result to the destination register. The value of the second source register modulo the register size in bits gives the number of
    /// bits by which the first source register is right-shifted.
    pub fn asrv(&mut self, sf: bool, m: usize, n: usize, d: usize) {
        let operand2 = self._get_op(m, sf);
        let rn = self._get_op(n, sf);

        let datasize = 32u64 << sf as u64;

        self._set_op(
            d,
            shared::shift_reg(rn, ShiftType::ASR, (operand2 % datasize) as usize),
            sf,
        );
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

    /// **B.cond**: Branch conditionally
    ///
    /// This instruction branches conditionally to a label at a PC-relative offset. This instruction provides a hint that this is not a
    /// subroutine call or return.
    pub fn b_cond(&mut self, imm19: u32, cond: u8) {
        let offset = shared::sint::<21>((imm19 << 2) as u64) as u64;
        if self.condition_holds(cond) {
            self.branch_to::<64>(self.pc + offset, BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **B**: Branch
    ///
    /// This instruction branches unconditionally to a label at a PC-relative offset. This instruction provides a hint that this is not a
    /// subroutine call or return.
    pub fn b(&mut self, imm26: u32) {
        let offset = shared::sint::<28>((imm26 << 2) as u64) as u64;
        self.branch_to::<64>(self.pc + offset, BranchType::DIR, false);
    }

    #[allow(unused_variables)]
    pub fn bc_cond(&mut self, imm19: u32, cond: u8) {
        todo!()
    }

    /// **BFC**: Bitfield clear
    ///
    /// This instruction sets a bitfield of &lt;width&gt; bits at bit position &lt;lsb&gt; of the destination register to zero, leaving the other
    /// destination bits unchanged.
    pub fn bfc(&mut self, sf: bool, N: bool, immr: u8, imms: u8, d: usize) {
        self.bfm(sf, N, immr, imms, 31, d)
    }

    /// **BFI**: Bitfield insert
    ///
    /// This instruction copies a bitfield of &lt;width&gt; bits from the least significant bits of the source register to bit position &lt;lsb&gt;
    /// of the destination register, leaving the other destination bits unchanged.
    pub fn bfi(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: usize, d: usize) {
        self.bfm(sf, N, immr, imms, n, d)
    }

    /// **BFM**: Bitfield move
    ///
    /// This instruction is usually accessed using one of its aliases, which are always preferred for disassembly.
    ///
    /// If &lt;imms&gt; is greater than or equal to &lt;immr&gt;, this copies a bitfield of (&lt;imms&gt;-&lt;immr&gt;+1) bits starting from bit position
    /// &lt;immr&gt; in the source register to the least significant bits of the destination register.
    ///
    /// If &lt;imms&gt; is less than &lt;immr&gt;, this copies a bitfield of (&lt;imms&gt;+1) bits from the least significant bits of the source register
    /// to bit position (regsize-&lt;immr&gt;) of the destination register, where regsize is the destination register size of 32 or 64 bits.
    ///
    /// In both cases, the other bits of the destination register remain unchanged.
    pub fn bfm(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: usize, d: usize) {
        let (wmask, tmask) = if sf {
            decode_bit_masks::<64>(N, imms, immr)
        } else {
            decode_bit_masks::<32>(N, imms, immr)
        };

        let src = if n == 31 { self._get_op(n, sf) } else { 0 };
        let dst = self._get_op(d, sf);

        let bot = (dst & !wmask) | (shared::ror(src, immr as u32, 32u32 << (sf as u32)) & wmask);
        self._set_op(d, (dst & !tmask) | (bot & tmask), sf);
    }

    /// **BFXIL**: Bitfield extract and insert at low end
    ///
    /// This instruction copies a bitfield of &lt;width&gt; bits starting from bit position &lt;lsb&gt; in the source register to the least
    /// significant bits of the destination register, leaving the other destination bits unchanged.
    pub fn bfxil(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: usize, d: usize) {
        self.bfm(sf, N, immr, imms, n, d)
    }

    /// **BIC (shifted register)**: Bitwise bit clear (shifted register)
    ///
    /// This instruction performs a bitwise AND between a register value and the complement of an optionally-shifted register
    /// value, and writes the result to the destination register.
    #[allow(unused_variables)]
    pub fn bic(&mut self, sf: bool, shift: u8, m: usize, imm6: u8, n: usize, d: usize) {
        todo!()
    }

    /// **BICS (shifted register)**: Bitwise bit clear (shifted register), setting flags
    ///
    /// This instruction performs a bitwise AND between a register value and the complement of an optionally-shifted register
    /// value, and writes the result to the destination register. It updates the condition flags based on the result.
    #[allow(unused_variables)]
    pub fn bics(&mut self, sf: bool, shift: u8, m: usize, imm6: u8, n: usize, d: usize) {
        todo!()
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
}
