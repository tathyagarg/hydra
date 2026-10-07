use crate::cpu::{
    Cpu, DecodeType, ExtendType, Feature, Register, ShiftType,
    common::{n_bits, ones},
    shared,
};

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
        if !Cpu::is_feature_implemented(Feature::CSSC) {
            self.end_of_decode(DecodeType::UNDEF);
        }

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
    /// cpu.add_er(true, 1, 0b000, 3, 0, 2);
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
    /// cpu.add_er(true, 1, 0b000, 3, 0, 2);
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
    /// cpu.add_er(true, 1, 0b011, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 4001);
    /// ```
    ///
    /// Without the masking to byte-width, we get 4001 instead.
    pub fn add_er(
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
    /// cpu.add_imm(true, false, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 11);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// // `ADD X1, X0, #10 LSL #12`
    /// cpu.add_imm(true, true, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 40961);
    /// ```
    pub fn add_imm(&mut self, sf: bool, sh: bool, imm12: u16, n: Register, d: Register) {
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
    /// cpu.add_sr(true, 0b00, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 17);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, 36);
    /// // `ADD X2, X0, X1, LSR #3`
    /// cpu.add_sr(true, 0b01, 1, 3, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 5);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1);
    /// cpu.__dbg_set_x(1, -44i64 as u64);
    /// // `ADD X2, X0, X1, ASR #2`
    /// cpu.add_sr(true, 0b10, 1, 2, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), -10i64 as u64);
    /// ```
    pub fn add_sr(&mut self, sf: bool, shift: u8, m: Register, imm6: u8, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let mval = self._get_op(m, sf);

        let shift_type = ShiftType::decode(shift).unwrap();

        let operand2 = if sf {
            shared::shift_reg::<64>(mval, shift_type, imm6 as usize)
        } else {
            shared::shift_reg::<32>(mval, shift_type, imm6 as usize)
        };

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
    /// cpu.adds_er(true, 1, 0b011, 0b010, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 1);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn adds_er(
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
    /// cpu.adds_imm(true, false, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 9);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, u64::MAX);
    /// // `ADDS X1, X0, #10 LSL #12`
    /// cpu.adds_imm(true, true, 10, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 40959);
    /// assert!(cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn adds_imm(&mut self, sf: bool, sh: bool, imm12: u16, n: Register, d: Register) {
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
    /// This does NOT overflow
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 8);
    /// cpu.__dbg_set_x(1, u64::MAX >> 3);
    /// // `ADDS X2, X0, X1, LSL #3`
    /// cpu.adds_sr(true, 0b00, 1, 3, 0, 2);
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
    /// cpu.adds_sr(true, 0b01, 1, 3, 0, 2);
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
    /// cpu.adds_sr(true, 0b10, 1, 2, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), -11i64 as u64);
    /// assert!(!cpu.__dbg_get_pstate().__dbg_get_C());
    /// ```
    pub fn adds_sr(
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

        let operand2 = if sf {
            shared::shift_reg::<64>(mval, shift_type, imm6 as usize)
        } else {
            shared::shift_reg::<32>(mval, shift_type, imm6 as usize)
        };

        let (res, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, false)
        } else {
            shared::add_with_carry::<32>(operand1, operand2, false)
        };

        self._set_op(d, res, sf);
        self.pstate.set_nzcv(&nzcv);
    }

    /// **ASRV**: Arithmetic shift right variable
    ///
    /// This instruction shifts a register value right by a variable number of bits, shifting in copies of its sign bit, and writes the
    /// result to the destination register. The value of the second source register modulo the register size in bits gives the number of
    /// bits by which the first source register is right-shifted.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 3284398546);
    /// cpu.__dbg_set_x(1, 12);
    /// cpu.asrv(false, 1, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 4294720575);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, -1234000707i32 as u64);
    /// cpu.__dbg_set_x(1, 12);
    /// cpu.asrv(false, 1, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 4294666026u64);
    /// ```
    pub fn asrv(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand2 = self._get_op(m, sf);
        let rn = self._get_op(n, sf);

        let datasize = 32 << sf as u32;

        self._set_op(
            d,
            if sf {
                shared::shift_reg::<64>(rn, ShiftType::ASR, (operand2 & ones(datasize)) as usize)
            } else {
                shared::shift_reg::<32>(rn, ShiftType::ASR, (operand2 & ones(datasize)) as usize)
            },
            sf,
        );
    }

    /// **LSLV**: Logical shift left variable
    ///
    /// This instruction shifts a register value left by a variable number of bits, shifting in zeros, and writes the result to the
    /// destination register. The value of the second source register modulo the register size in bits gives the number of bits by
    /// which the first source register is left-shifted.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 338);
    /// cpu.__dbg_set_x(1, 12);
    /// cpu.lslv(false, 1, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 1384448);
    /// ```
    pub fn lslv(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand2 = self._get_op(m, sf);
        let rn = self._get_op(n, sf);

        let datasize = 32 << sf as u32;

        self._set_op(
            d,
            if sf {
                shared::shift_reg::<64>(rn, ShiftType::LSL, (operand2 & ones(datasize)) as usize)
            } else {
                shared::shift_reg::<32>(rn, ShiftType::LSL, (operand2 & ones(datasize)) as usize)
            },
            sf,
        );
    }

    /// **LSRV**: Logical shift right variable
    ///
    /// This instruction shifts a register value right by a variable number of bits, shifting in zeros, and writes the result to the
    /// destination register. The value of the second source register modulo the register size in bits gives the number of bits by
    /// which the first source register is right-shifted.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 3284398546);
    /// cpu.__dbg_set_x(1, 12);
    /// cpu.lsrv(false, 1, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 801855);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, -1234000707i32 as u64);
    /// cpu.__dbg_set_x(1, 12);
    /// cpu.lsrv(false, 1, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 747306);
    /// ```
    pub fn lsrv(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand2 = self._get_op(m, sf);
        let rn = self._get_op(n, sf);

        let datasize = 32 << sf as u32;

        self._set_op(
            d,
            if sf {
                shared::shift_reg::<64>(rn, ShiftType::LSR, (operand2 & ones(datasize)) as usize)
            } else {
                shared::shift_reg::<32>(rn, ShiftType::LSR, (operand2 & ones(datasize)) as usize)
            },
            sf,
        );
    }

    /// **MADD**: Multiply-add
    ///
    /// This instruction multiplies two register values, adds a third register value, and writes the result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 10);
    /// cpu.__dbg_set_x(1, 3);
    /// cpu.__dbg_set_x(2, 5);
    /// cpu.madd(false, 0, 1, 2, 3);
    /// assert_eq!(cpu.__dbg_get_x(3), 53);
    /// ```
    pub fn madd(&mut self, sf: bool, m: Register, a: Register, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let operand2 = self._get_op(m, sf);
        let operand3 = if a < 31 { self._get_op(a, sf) } else { 0 };

        let result = operand3 + (operand1 * operand2);
        let datasize = 32 << sf as u32;

        self._set_op(d, result & ones(datasize), sf);
    }

    /// **MSUB**: Multiply-subtract
    ///
    /// This instruction multiplies two register values, subtracts the product from a third register value, and writes the result to the
    /// destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 10);
    /// cpu.__dbg_set_x(1, 3);
    /// cpu.__dbg_set_x(2, 5);
    /// cpu.msub(false, 0, 1, 2, 3);
    /// assert_eq!(cpu.__dbg_get_x(3), (-47i32) as u32 as u64);
    /// ```
    pub fn msub(&mut self, sf: bool, m: Register, a: Register, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let operand2 = self._get_op(m, sf);
        let operand3 = if a < 31 { self._get_op(a, sf) } else { 0 };

        let result = operand3.wrapping_sub(operand1 * operand2);
        let datasize = 32 << sf as u32;

        self._set_op(d, result & ones(datasize), sf);
    }

    /// **RORV**: Rotate right variable
    ///
    /// This instruction provides the value of the contents of a register rotated by a variable number of bits. The bits that are rotated
    /// off the right end are inserted into the vacated bit positions on the left. The value of the second source register modulo the
    /// register size in bits gives the number of bits by which the first source register is right-shifted.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 18);
    /// cpu.__dbg_set_x(1, 12);
    /// cpu.rorv(false, 1, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 18874368);
    /// ```
    pub fn rorv(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand2 = self._get_op(m, sf);
        let rn = self._get_op(n, sf);

        let datasize = 32 << sf as u32;

        self._set_op(
            d,
            if sf {
                shared::shift_reg::<64>(rn, ShiftType::ROR, (operand2 & ones(datasize)) as usize)
            } else {
                shared::shift_reg::<32>(rn, ShiftType::ROR, (operand2 & ones(datasize)) as usize)
            },
            sf,
        );
    }

    /// **SBC**: Subtract with carry
    ///
    /// This instruction subtracts a register value and the value of NOT (Carry flag) from a register value, and writes the result to
    /// the destination register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 10);
    /// cpu.__dbg_set_x(1, 2);
    /// cpu.__dbg_get_pstate().__dbg_set_C(true);
    /// cpu.sbc(true, 1, 0, 2);  // `SBC X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 8);
    /// ```
    pub fn sbc(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let operand2 = !self._get_op(m, sf);

        let (result, _) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C())
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C())
        };

        self._set_op(d, result, sf);
    }

    /// **SBCS**: Subtract with carry, setting flags
    ///
    /// This instruction subtracts a register value and the value of NOT (Carry flag) from a register value, and writes the result to
    /// the destination register. It updates the condition flags based on the result.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, common::NZCV};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 10);
    /// cpu.__dbg_set_x(1, 2);
    /// cpu.__dbg_get_pstate().__dbg_set_C(true);
    /// cpu.sbcs(true, 1, 0, 2);  // `SBCS X2, X0, X1`
    /// assert_eq!(cpu.__dbg_get_x(2), 8);
    /// assert_eq!(cpu.__dbg_get_pstate().nzcv(), NZCV::init(false, false, true, false));
    /// ```
    pub fn sbcs(&mut self, sf: bool, m: Register, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let operand2 = !self._get_op(m, sf);

        let (result, nzcv) = if sf {
            shared::add_with_carry::<64>(operand1, operand2, self.pstate.C())
        } else {
            shared::add_with_carry::<32>(operand1, operand2, self.pstate.C())
        };

        self._set_op(d, result, sf);
        self.pstate.set_nzcv(&nzcv);
    }
}
