use crate::cpu::{Cpu, ShiftType, shared};

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
    pub fn _and_imm(&mut self, sf: bool, imm_n: bool, immr: u8, imms: u8, n: usize, d: usize) {
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
    pub fn _ands_imm(&mut self, sf: bool, imm_n: bool, immr: u8, imms: u8, n: usize, d: usize) {
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
}
