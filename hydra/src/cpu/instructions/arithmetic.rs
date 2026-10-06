use crate::cpu::{Cpu, ExtendType, Register, ShiftType, common::n_bits, shared};

impl Cpu {
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
    pub fn abs(&mut self, sf: bool, n: Register, d: Register) {
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
    pub fn adc(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let operand2 = self._get_op(m, sf);

        let (result, _) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C())
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C())
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
    pub fn adcs(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let operand2 = self._get_op(m, sf);

        let (result, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C())
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C())
        };

        self._set_op(d, result, sf);
        self.pstate.set_nzcv(&nzcv);
    }

    /// **ADD (extended register)**: Add extended and scaled register
    ///
    /// This instruction adds a register value and a sign-extended or zero-extended register value, followed by an optional left shift
    /// amount, and writes the result to the destination register. The argument that is extended from the &lt;Rm&gt; register can be a byte,
    /// halfword, word, or doubleword.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// // `ADD X2, X0, X1, UXTB #3`
    /// cpu._add_er(true, 1, 0b000, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 17);
    /// ```
    ///
    /// We get 17 because 17 = 1 + (2 << 3)
    /// The ExtendType is not very consequential because we are dealing with such small values.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 500);
    /// // `ADD X2, X0, X1, UXTB #3`
    /// cpu._add_er(true, 1, 0b000, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 1953);
    /// ```
    ///
    /// We get 1953 because 1953 = 1 + (244 << 3), where 244 = 500 & 0xFF (due to UXTB extending)
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 500);
    /// // `ADD X2, X0, X1, UXTD #3`
    /// cpu._add_er(true, 1, 0b011, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 4001);
    /// ```
    ///
    /// Without the masking to byte-width, we get 4001 instead.
    pub fn _add_er(
        &mut self,
        sf: bool,
        m: Register,
        option: u8,
        imm3: u8,
        n: Register,
        d: Register,
    ) {
        let operand1 = self._get_reg_or_sp(n, sf);

        let mval = self._get_op(m, option & 0b11 == 0b11);
        let extend_type = ExtendType::decode(option).unwrap();

        let (result, _) = if sf {
            let operand2 = shared::extend_reg::<64>(mval, extend_type, imm3);
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            let operand2 = shared::extend_reg::<32>(mval, extend_type, imm3);
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        self._set_reg_or_sp(d, result, sf);
    }

    /// **ADD (Immediate)**: Add immediate value
    ///
    /// This instruction adds a register value and an optionally-shifted immediate value, and writes the result to the destination
    /// register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// // `ADD X1, X0, #10 LSL #0`
    /// cpu._add_imm(true, false, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 11);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// // `ADD X1, X0, #10 LSL #12`
    /// cpu._add_imm(true, true, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 40961);
    /// ```
    pub fn _add_imm(&mut self, sf: bool, sh: bool, imm12: u16, n: Register, d: Register) {
        let imm12 = n_bits(imm12, 12);
        let imm = if sh { imm12 << 12 } else { imm12 };

        let operand1 = self._get_reg_or_sp(n, sf);

        let (result, _) = if sf {
            shared::add_with_carry::<64>(operand1, imm, false)
        } else {
            shared::add_with_carry::<32>(operand1, imm, false)
        };

        self._set_reg_or_sp(d, result, sf);
    }

    /// **ADD (Shifted Register)**: Add optionally-shifted register
    ///
    /// This instruction adds a register value and an optionally-shifted register value, and writes the result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 2);
    /// // `ADD X2, X0, X1, LSL #3`
    /// cpu._add_sr(true, 0b00, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 17);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 36);
    /// // `ADD X2, X0, X1, LSR #3`
    /// cpu._add_sr(true, 0b01, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 5);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, -44i64 as u64);
    /// // `ADD X2, X0, X1, ASR #2`
    /// cpu._add_sr(true, 0b10, 1, 2, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), -10i64 as u64);
    /// ```
    pub fn _add_sr(
        &mut self,
        sf: bool,
        shift: u8,
        m: Register,
        imm6: u8,
        n: Register,
        d: Register,
    ) {
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
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, u64::MAX >> 2);
    /// // `ADDS X2, X0, X1, UXTB #3`
    /// cpu._adds_er(true, 1, 0b011, 0b010, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 1);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn _adds_er(
        &mut self,
        sf: bool,
        m: Register,
        option: u8,
        imm3: u8,
        n: Register,
        d: Register,
    ) {
        let operand1 = self._get_reg_or_sp(n, sf);

        let mval = self._get_op(m, option & 0b11 == 0b11);
        let extend_type = ExtendType::decode(option).unwrap();

        let (result, nzcv) = if sf {
            let operand2 = shared::extend_reg::<64>(mval, extend_type, imm3);
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            let operand2 = shared::extend_reg::<32>(mval, extend_type, imm3);
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        self.pstate.set_nzcv(&nzcv);
        self._set_reg_or_sp(d, result, sf);
    }

    /// **ADDS (Immediate)**: Add immediate value, setting flags
    ///
    /// This instruction adds a register value and an optionally-shifted immediate value, and writes the result to the destination
    /// register. It updates the condition flags based on the result.
    ///
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// // `ADDS X1, X0, #10 LSL #0`
    /// cpu._adds_imm(true, false, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 9);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// // `ADDS X1, X0, #10 LSL #12`
    /// cpu._adds_imm(true, true, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 40959);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn _adds_imm(&mut self, sf: bool, sh: bool, imm12: u16, n: Register, d: Register) {
        let imm12 = n_bits(imm12, 12);
        let imm = if sh { imm12 << 12 } else { imm12 };

        let operand1 = self._get_reg_or_sp(n, sf);

        let (result, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, imm, false)
        } else {
            shared::add_with_carry::<32>(operand1, imm, false)
        };

        self.pstate.set_nzcv(&nzcv);
        self._set_reg_or_sp(d, result, sf);
    }

    /// **ADDS (Shifted Register)**: Add optionally-shifted register, setting flags
    ///
    /// This instruction adds a register value and an optionally-shifted register value, and writes the result to the destination register.
    /// It updates the condition flags based on the result.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
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
    /// # use hydra::cpu::{Cpu, ExtendType};
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
    /// # use hydra::cpu::{Cpu, ExtendType};
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
    pub fn _adds_sr(
        &mut self,
        sf: bool,
        shift: u8,
        m: Register,
        imm6: u8,
        n: Register,
        d: Register,
    ) {
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
        self.pstate.set_nzcv(&nzcv);
    }

    /// **ASR (register)**: Arithmetic shift right (register)
    ///
    /// This instruction shifts a register value right by a variable number of bits, shifting in copies of its sign bit, and writes the
    /// result to the destination register. The value of the second source register modulo the register size in bits gives the number of
    /// bits by which the first source register is right-shifted.
    pub fn _asr_reg(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        self.asrv(sf, m, n, d);
    }

    /// **ASR (immediate)**: Arithmetic shift right (immediate)
    ///
    /// This instruction shifts a register value right by an immediate number of bits, shifting in copies of the sign bit in the upper
    /// bits and zeros in the lower bits, and writes the result to the destination register.
    #[allow(unused_variables)]
    pub fn _asr_imm(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: Register, d: Register) {
        todo!("ASR (immediate) is an alias of SBFM which is yet to be implemented")
    }

    /// **ASRV**: Arithmetic shift right variable
    ///
    /// This instruction shifts a register value right by a variable number of bits, shifting in copies of its sign bit, and writes the
    /// result to the destination register. The value of the second source register modulo the register size in bits gives the number of
    /// bits by which the first source register is right-shifted.
    pub fn asrv(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand2 = self._get_op(m, sf);
        let rn = self._get_op(n, sf);

        let datasize = 32u64 << sf as u64;

        self._set_op(
            d,
            shared::shift_reg(rn, ShiftType::ASR, (operand2 % datasize) as usize),
            sf,
        );
    }
}
