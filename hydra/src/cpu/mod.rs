#![allow(dead_code)]

mod pstate;
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
            pstate: pstate::PState::default(),
        }
    }
}

impl Cpu {
    fn _to_op(&self, n: usize, sf: bool) -> u64 {
        if sf {
            self.x[n]
        } else {
            self.x[n] as u32 as u64
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
    /// cpu.abs(true, 1, 0);  // `ABS X1, X0`
    /// assert_eq!(cpu.__dbg_get_x(1), 5);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 9223372032559808513u64);
    /// cpu.abs(false, 1, 0);  // `ABS W1, W0`
    /// assert_eq!(cpu.__dbg_get_x(1), 1);
    /// ```
    pub fn abs(&mut self, sf: bool, d: usize, n: usize) {
        let operand1 = self._to_op(n, sf) as i64;

        let result = operand1.abs() as u64;
        self.x[d] = result;
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
    /// cpu.adc(true, 2, 0, 1);  // `ADC X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 3);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// cpu.__dbg_get_pstate().__dbg_set_C(true);  // Set PSTATE.C = 1
    /// cpu.adc(true, 2, 0, 1);  // `ADC X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 4);
    /// ```
    pub fn adc(&mut self, sf: bool, d: usize, n: usize, m: usize) {
        let operand1 = self._to_op(n, sf);
        let operand2 = self._to_op(m, sf);

        let (result, _) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C)
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C)
        };

        self.x[d] = result;
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
    /// cpu.adcs(true, 2, 0, 1);  // `ADCS X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 3);
    /// assert!(!cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// cpu.__dbg_set_x(1, 1);
    /// cpu.adcs(true, 2, 0, 1);  // `ADCS X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 0);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn adcs(&mut self, sf: bool, d: usize, n: usize, m: usize) {
        let operand1 = self._to_op(n, sf);
        let operand2 = self._to_op(m, sf);

        let (result, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C)
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C)
        };

        self.x[d] = result;
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
    /// // ADD X2, X0, X1, UXTB #3
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
            self.x[rn]
        } else {
            self.sp
        };

        let mval = if option & 0b11 == 0b11 {
            self.x[m]
        } else {
            self.x[m] as u32 as u64
        };

        let extend_type = ExtendType::decode(option).unwrap();

        if imm3 > 4 {
            panic!("imm3 too large");
        }
        let shift = imm3;

        let (result, _) = if sf {
            let operand2 = shared::extend_reg::<64>(mval, extend_type, shift);
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            let operand2 = shared::extend_reg::<32>(mval, extend_type, shift);
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        if let RegisterSP::Register(rd) = d {
            self.x[rd] = result;
        } else {
            self.sp = result;
        }
    }
}
