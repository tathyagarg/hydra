use crate::cpu::{Cpu, DecodeType, Feature, REG_SP, Register, ShiftType, common::*, shared};

impl Cpu {
    /// **AND (immediate)**: Bitwise AND (immediate)
    ///
    /// This instruction performs a bitwise AND between a register value and an immediate value, and writes the result to the
    /// destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0x1234_5678_9ABC_DEF0);
    /// // `AND X1, X0, #0xFF`
    /// cpu.and_imm(true, true, 0b000000, 0b000111, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0xF0);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFFFF_FFFF_FFFF_FFFF);
    /// // `AND X1, X0, #0x00`
    /// cpu.and_imm(true, false, 0b000000, 0b110100, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0x1F1F_1F1F_1F1F_1F1F);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 232131);
    /// // `AND X1, X0, #0x00`
    /// cpu.and_imm(true, false, 0b001101, 0b101011, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 2752);
    /// ```
    ///
    /// Special thanks to <https://gist.github.com/dinfuehr/51a01ac58c0b23e4de9aac313ed6a06a>
    pub fn and_imm(&mut self, sf: bool, imm_n: bool, immr: u8, imms: u8, n: Register, d: Register) {
        let (imm, _) = if sf {
            shared::decode_bit_masks::<64>(imm_n, imms, immr)
        } else {
            shared::decode_bit_masks::<32>(imm_n, imms, immr)
        };

        let operand1 = self._get_op(n, sf);
        let operand2 = imm;

        let result = operand1 & operand2;

        self._set_reg_or_sp(d, result, sf);
    }

    /// **AND (Shifted Register)**: Bitwise AND (shifted register)
    ///
    /// This instruction performs a bitwise AND between a register value and an optionally-shifted register value, and writes the
    /// result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 213);
    /// cpu.__dbg_set_x(1, 82);
    /// cpu.and_sr(true, 0b00, 1, 0, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 80);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 213);
    /// cpu.__dbg_set_x(1, 5);
    /// cpu.and_sr(true, 0b00, 1, 4, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 80);
    /// ```
    pub fn and_sr(&mut self, sf: bool, shift: u8, m: Register, imm6: u8, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let operand2 = if sf {
            shared::shift_reg::<64>(
                self._get_op(m, sf),
                ShiftType::decode(shift).unwrap(),
                imm6 as usize,
            )
        } else {
            shared::shift_reg::<32>(
                self._get_op(m, sf),
                ShiftType::decode(shift).unwrap(),
                imm6 as usize,
            )
        };

        self._set_op(d, operand1 & operand2, sf);
    }

    /// **ANDS (Immediate)**: Bitwise AND (immediate), setting flags
    ///
    /// This instruction performs a bitwise AND between a register value and an immediate value, and writes the result to the
    /// destination register. It updates the condition flags based on the result.
    pub fn ands_imm(
        &mut self,
        sf: bool,
        imm_n: bool,
        immr: u8,
        imms: u8,
        n: Register,
        d: Register,
    ) {
        let (imm, _) = if sf {
            shared::decode_bit_masks::<64>(imm_n, imms, immr)
        } else {
            shared::decode_bit_masks::<32>(imm_n, imms, immr)
        };

        let operand1 = self._get_op(n, sf);
        let operand2 = imm;

        let result = operand1 & operand2;

        self._set_reg_or_sp(d, result, sf);
        self.pstate.set_nzcv(&NZCV::init(
            (result >> ((32 << sf as u64) - 1)) == 1,
            result == 0,
            false,
            false,
        ));
    }

    /// **ANDS: (shifted register)**: Bitwise AND (shifted register), setting flags
    ///
    /// This instruction performs a bitwise AND between a register value and an optionally-shifted register value, and writes the
    /// result to the destination register. It updates the condition flags based on the result.
    pub fn ands_sr(
        &mut self,
        sf: bool,
        shift: u8,
        m: Register,
        imm6: u8,
        n: Register,
        d: Register,
    ) {
        let operand1 = self._get_op(n, sf);
        let operand2 = if sf {
            shared::shift_reg::<64>(
                self._get_op(m, sf),
                ShiftType::decode(shift).unwrap(),
                imm6 as usize,
            )
        } else {
            shared::shift_reg::<32>(
                self._get_op(m, sf),
                ShiftType::decode(shift).unwrap(),
                imm6 as usize,
            )
        };

        let result = operand1 & operand2;
        self._set_op(d, result, sf);
        self.pstate.set_nzcv(&NZCV::init(
            (result >> ((32 << sf as u64) - 1)) == 1,
            result == 0,
            false,
            false,
        ));
    }

    /// **BFC**: Bitfield clear
    ///
    /// This instruction sets a bitfield of &lt;width&gt; bits at bit position &lt;lsb&gt; of the destination register to zero, leaving the other
    /// destination bits unchanged.
    pub fn bfc(&mut self, sf: bool, N: bool, immr: u8, imms: u8, d: Register) {
        self.bfm(sf, N, immr, imms, 31, d)
    }

    /// **BFI**: Bitfield insert
    ///
    /// This instruction copies a bitfield of &lt;width&gt; bits from the least significant bits of the source register to bit position &lt;lsb&gt;
    /// of the destination register, leaving the other destination bits unchanged.
    pub fn bfi(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: Register, d: Register) {
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
    pub fn bfm(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: Register, d: Register) {
        if sf && !N {
            self.end_of_decode(DecodeType::UNDEF);
        }

        if !sf && (N || nth_bit(immr, 5) || nth_bit(imms, 5)) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let (wmask, tmask) = if sf {
            shared::decode_bit_masks::<64>(N, imms, immr)
        } else {
            shared::decode_bit_masks::<32>(N, imms, immr)
        };

        let src = if n == REG_SP { self._get_op(n, sf) } else { 0 };
        let dst = self._get_op(d, sf);

        let bot = (dst & !wmask) | (shared::ror(src, immr as u32, 32u32 << (sf as u32)) & wmask);
        self._set_op(d, (dst & !tmask) | (bot & tmask), sf);
    }

    /// **BFXIL**: Bitfield extract and insert at low end
    ///
    /// This instruction copies a bitfield of &lt;width&gt; bits starting from bit position &lt;lsb&gt; in the source register to the least
    /// significant bits of the destination register, leaving the other destination bits unchanged.
    pub fn bfxil(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: Register, d: Register) {
        self.bfm(sf, N, immr, imms, n, d)
    }

    /// **BIC (shifted register)**: Bitwise bit clear (shifted register)
    ///
    /// This instruction performs a bitwise AND between a register value and the complement of an optionally-shifted register
    /// value, and writes the result to the destination register.
    #[allow(unused_variables)]
    pub fn bic(&mut self, sf: bool, shift: u8, m: Register, imm6: u8, n: Register, d: Register) {
        todo!()
    }

    /// **BICS (shifted register)**: Bitwise bit clear (shifted register), setting flags
    ///
    /// This instruction performs a bitwise AND between a register value and the complement of an optionally-shifted register
    /// value, and writes the result to the destination register. It updates the condition flags based on the result.
    #[allow(unused_variables)]
    pub fn bics(&mut self, sf: bool, shift: u8, m: Register, imm6: u8, n: Register, d: Register) {
        todo!()
    }

    /// **CFINV**: Invert carry flag
    ///
    /// This instruction inverts the value of the PSTATE.C flag.
    pub fn cfinv(&mut self) {
        if !Cpu::is_feature_implemented(Feature::FlagM) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        self.pstate.invert_C();
    }

    /// **CHKFEAT**: Check feature status
    ///
    /// This instruction indicates the status of features. For more information, see Check Feature.
    /// If FEAT_CHK is not implemented, this instruction executes as a NOP.
    pub fn chkfeat(&mut self) {
        todo!()
    }

    /// **CLS**: Count leading sign bits
    ///
    /// This instruction determines the number of leading bits of the source register that have the same value as the most significant
    /// bit of the register, and writes the result to the destination register. The result does not include the most significant bit of the
    /// source register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFF00_0000_0000_0000);
    /// cpu.cls(true, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 7);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFF00_0000);
    /// cpu.cls(false, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 7);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0x0000_FFFF_FFFF_FFFF);
    /// cpu.cls(true, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 15);
    /// ```
    pub fn cls(&mut self, sf: bool, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let result = if sf {
            count_leading_sign_bits::<64>(operand1)
        } else {
            count_leading_sign_bits::<32>(operand1)
        };

        self._set_op(d, result as u64, sf);
    }

    /// **CLZ**: Count leading zeros
    ///
    /// This instruction determines the number of consecutive binary zero bits, starting from the most significant bit in the source
    /// register, and places the result in the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFF00_0000_0000_0000);
    /// cpu.clz(true, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0x0000_FFFF_FFFF_FFFF);
    /// cpu.clz(true, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 16);
    /// ```
    pub fn clz(&mut self, sf: bool, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let result = if sf {
            count_leading_zero_bits::<64>(operand1)
        } else {
            count_leading_zero_bits::<32>(operand1)
        };

        self._set_op(d, result as u64, sf);
    }

    /// **CNT**: Count bits
    ///
    /// This instruction determines the number of binary one bits in the value of the source register, and writes the result to the
    /// destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0);
    /// cpu.cnt(true, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0x1234_5678_9ABC_DEF0);
    /// cpu.cnt(true, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 32);
    /// ```
    pub fn cnt(&mut self, sf: bool, n: Register, d: Register) {
        if !Cpu::is_feature_implemented(Feature::CSSC) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let operand1 = self._get_op(n, sf);
        let result = operand1.count_ones();

        self._set_op(d, result as u64, sf);
    }

    /// **CSEL**: Conditional select
    ///
    /// This instruction writes the value of the first source register to the destination register if the condition is TRUE. If the
    /// condition is FALSE, it writes the value of the second source register to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(true);
    /// cpu.csel(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 7);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(false);
    /// cpu.csel(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 5);
    /// ```
    pub fn csel(&mut self, sf: bool, m: Register, cond: u8, n: Register, d: Register) {
        let result = if self.condition_holds(cond) {
            self._get_op(n, sf)
        } else {
            self._get_op(m, sf)
        };

        self._set_op(d, result, sf);
    }

    /// **CSINC**: Conditional select increment
    ///
    /// This instruction returns, in the destination register, the value of the first source register if the condition is TRUE, and
    /// otherwise returns the value of the second source register incremented by 1.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(true);
    /// cpu.csinc(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 7);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(false);
    /// cpu.csinc(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 6);
    /// ```
    pub fn csinc(&mut self, sf: bool, m: Register, cond: u8, n: Register, d: Register) {
        let result = if self.condition_holds(cond) {
            self._get_op(n, sf)
        } else {
            self._get_op(m, sf) + 1
        };

        self._set_op(d, result, sf);
    }

    /// **CSINV**: Conditional select invert
    ///
    /// This instruction returns, in the destination register, the value of the first source register if the condition is TRUE, and
    /// otherwise returns the bitwise inversion value of the second source register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(true);
    /// cpu.csinv(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 7);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(false);
    /// cpu.csinv(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 0xFFFF_FFFF_FFFF_FFFA);
    /// ```
    pub fn csinv(&mut self, sf: bool, m: Register, cond: u8, n: Register, d: Register) {
        let result = if self.condition_holds(cond) {
            self._get_op(n, sf)
        } else {
            !self._get_op(m, sf)
        };

        self._set_op(d, result, sf);
    }

    /// **CSNEG**: Conditional select negation
    ///
    /// This instruction returns, in the destination register, the value of the first source register if the condition is TRUE, and
    /// otherwise returns the negated value of the second source register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(true);
    /// cpu.csneg(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 7);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.__dbg_set_x(1, 7);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(false);
    /// cpu.csneg(true, 0, 0b0000, 1, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 0xFFFF_FFFF_FFFF_FFFB);
    /// ```
    pub fn csneg(&mut self, sf: bool, m: Register, cond: u8, n: Register, d: Register) {
        let result = if self.condition_holds(cond) {
            self._get_op(n, sf)
        } else {
            !self._get_op(m, sf) + 1
        };

        self._set_op(d, result, sf);
    }

    /// **CTZ**: Count trailing zeros
    ///
    /// This instruction determines the number of consecutive binary zero bits, starting from the least significant bit in the source
    /// register, and places the result in the destination register.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 5);
    /// cpu.ctz(false, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFFFF_0000_0000_0000);
    /// cpu.ctz(true, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 48);
    /// ```
    pub fn ctz(&mut self, sf: bool, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let result = if sf {
            count_leading_zero_bits::<64>(operand1.reverse_bits())
        } else if operand1 == 0 {
            32
        } else {
            count_leading_zero_bits::<32>((operand1 as u32).reverse_bits() as u64)
        };

        self._set_op(d, result as u64, sf);
    }

    /// **EON (shifted register)**: Bitwise exclusive-OR NOT (shifted register)
    ///
    /// This instruction performs a bitwise exclusive-OR NOT between a register value and an optionally-shifted register value,
    /// and writes the result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 123);
    /// cpu.__dbg_set_x(1, 6);
    /// cpu.eon_sr(false, 0b00, 1, 2, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 0x0000_0000_FFFF_FF9C);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 123);
    /// cpu.__dbg_set_x(1, 0xFFFF_FF8E);
    /// cpu.eon_sr(false, 0b00, 1, 0, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 10);
    /// ```
    pub fn eon_sr(&mut self, sf: bool, shift: u8, m: Register, imm6: u8, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let mval = self._get_op(m, sf);

        let shift_type = ShiftType::decode(shift).unwrap();

        let operand2 = if sf {
            shared::shift_reg::<64>(mval, shift_type, imm6 as usize)
        } else {
            shared::shift_reg::<32>(mval, shift_type, imm6 as usize)
        };

        let result = if sf {
            operand1 ^ !operand2
        } else {
            ((operand1 as u32) ^ (!operand2 as u32)) as u64
        };

        self._set_op(d, result, sf);
    }

    /// **EOR (immediate)**: Bitwise exclusive-OR (immediate)
    ///
    /// This instruction performs a bitwise exclusive-OR between a register value and an immediate value, and writes the result to
    /// the destination register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 123);
    /// // EOR W1, W0, #0x3c
    /// cpu.eor_imm(false, true, 0b111110, 0b000011, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0x47);
    /// ```
    pub fn eor_imm(&mut self, sf: bool, N: bool, immr: u8, imms: u8, n: Register, d: Register) {
        let (imm, _) = if sf {
            shared::decode_bit_masks::<64>(N, imms, immr)
        } else {
            shared::decode_bit_masks::<32>(N, imms, immr)
        };

        let operand1 = self._get_op(n, sf);
        let result = if sf {
            operand1 ^ imm
        } else {
            ((operand1 as u32) ^ (imm as u32)) as u64
        };

        if d == REG_SP {
            self.sp = result;
        } else {
            self._set_op(d, result, sf);
        }
    }

    /// **EOR (shifted register)**: Bitwise exclusive-OR (shifted register)
    ///
    /// This instruction performs a bitwise exclusive-OR between a register value and an optionally-shifted register value, and
    /// writes the result to the destination register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 123);
    /// cpu.__dbg_set_x(1, 6);
    /// cpu.eor_sr(false, 0b00, 1, 2, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 0x63);
    /// ```
    pub fn eor_sr(&mut self, sf: bool, shift: u8, m: Register, imm6: u8, n: Register, d: Register) {
        let operand1 = self._get_op(n, sf);
        let mval = self._get_op(m, sf);

        let shift_type = ShiftType::decode(shift).unwrap();

        let operand2 = if sf {
            shared::shift_reg::<64>(mval, shift_type, imm6 as usize)
        } else {
            shared::shift_reg::<32>(mval, shift_type, imm6 as usize)
        };

        let result = if sf {
            operand1 ^ operand2
        } else {
            ((operand1 as u32) ^ (operand2 as u32)) as u64
        };

        self._set_op(d, result, sf);
    }

    /// **EXTR**: Extract register
    ///
    /// This instruction extracts a register from a pair of registers.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 1632208504);
    /// cpu.__dbg_set_x(1, 24810007);
    /// cpu.extr(false, false, 1, 8, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 0x78017a92);
    /// ```
    pub fn extr(&mut self, sf: bool, N: bool, m: Register, imms: u8, n: Register, d: Register) {
        if N != sf {
            self.end_of_decode(DecodeType::UNDEF);
        }
        if !sf && nth_bit(imms, 5) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let operand1 = self._get_op(n, sf);
        let operand2 = self._get_op(m, sf);

        let result = if sf {
            let concat: u128 = (operand1 as u128) << 64 | operand2 as u128;
            ((concat >> imms) & ones(64) as u128) as u64
        } else {
            let concat: u64 = (operand1 << 32) | operand2;
            (concat >> imms) & ones(32)
        };

        self._set_op(d, result, sf);
    }
}
