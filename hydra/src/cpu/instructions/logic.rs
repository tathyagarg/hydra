use crate::cpu::{Cpu, DecodeType, Feature, Register, ShiftType, common::*, shared};

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
    /// cpu._and_imm(true, true, 0b000000, 0b000111, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0xF0);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFFFF_FFFF_FFFF_FFFF);
    /// // `AND X1, X0, #0x00`
    /// cpu._and_imm(true, false, 0b000000, 0b110100, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 0x1F1F_1F1F_1F1F_1F1F);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 232131);
    /// // `AND X1, X0, #0x00`
    /// cpu._and_imm(true, false, 0b001101, 0b101011, 0, 1);
    /// assert_eq!(cpu.__dbg_get_x(1), 2752);
    /// ```
    ///
    /// Special thanks to <https://gist.github.com/dinfuehr/51a01ac58c0b23e4de9aac313ed6a06a>
    pub fn _and_imm(
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

        println!("{} {}", operand1, operand2);
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
    /// cpu._and_sr(true, 0b00, 1, 0, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 80);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 213);
    /// cpu.__dbg_set_x(1, 5);
    /// cpu._and_sr(true, 0b00, 1, 4, 0, 2);
    /// assert_eq!(cpu.__dbg_get_x(2), 80);
    /// ```
    pub fn _and_sr(
        &mut self,
        sf: bool,
        shift: u8,
        m: Register,
        imm6: u8,
        n: Register,
        d: Register,
    ) {
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

        println!("{} {}", operand1, operand2);
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
    pub fn _ands_sr(
        &mut self,
        sf: bool,
        shift: u8,
        m: Register,
        imm6: u8,
        n: Register,
        d: Register,
    ) {
        let operand1 = self._get_op(n, sf);
        let operand2 = shared::shift_reg(
            self._get_op(m, sf),
            ShiftType::decode(shift).unwrap(),
            imm6 as usize,
        );

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

        let src = if n == 31 { self._get_op(n, sf) } else { 0 };
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
}
