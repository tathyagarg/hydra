use crate::cpu::{BranchType, CmpOp, Cpu, Feature, Register, common::n_bits, shared};

impl Cpu {
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

    /// **CBB&lt;cc&gt;**: Compare bytes and branch
    ///
    /// This instruction compares the byte values in two registers, and conditionally branches to a label at a PC-relative offset if the
    /// condition is true. This instruction provides a hint that this is not a subroutine call or return. This instruction does not affect
    /// the condition flags.
    pub fn cbb_cc(&mut self, cc: u8, m: Register, H: bool, imm9: u16, t: Register) {
        if !Cpu::is_feature_implemented(Feature::CMPBR) {
            panic!("??");
        }

        let (op, unsigned) = match cc {
            0b000 => (CmpOp::GT, false),
            0b001 => (CmpOp::GE, false),
            0b010 => (CmpOp::GT, true),
            0b011 => (CmpOp::GE, true),
            0b110 => (CmpOp::EQ, true),
            0b111 => (CmpOp::NE, true),
            _ => unreachable!(),
        };

        let offset = shared::sint::<11>((imm9 as u64) << 2);

        let datasize = 8usize << (H as u8);
        let operand1 = n_bits(self.x[t], datasize);
        let operand2 = n_bits(self.x[m], datasize);

        let cond = if unsigned {
            let value1 = operand1;
            let value2 = operand2;

            match op {
                CmpOp::EQ => value1 == value2,
                CmpOp::NE => value1 != value2,
                CmpOp::GT => value1 > value2,
                CmpOp::GE => value1 >= value2,
                _ => unreachable!(),
            }
        } else {
            let value1 = operand1 as i64;
            let value2 = operand2 as i64;

            match op {
                CmpOp::EQ => value1 == value2,
                CmpOp::NE => value1 != value2,
                CmpOp::GT => value1 > value2,
                CmpOp::GE => value1 >= value2,
                _ => unreachable!(),
            }
        };

        if cond {
            self.branch_to::<64>(((self.pc as i64) + offset) as u64, BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CBBLE**: Compare signed less than or equal to bytes and branch
    ///
    /// This instruction compares the signed byte values in two registers, and conditionally branches to a label at a PC-relative
    /// offset if the second value is less than or equal to the first. This instruction provides a hint that this is not a subroutine call or
    /// return. This instruction does not affect the condition flags.
    pub fn cbb_le(&mut self, m: Register, H: bool, imm9: u16, t: Register) {
        self.cbb_cc(0b001, m, H, imm9, t)
    }

    /// **CBBLO**: Compare unsigned lower than bytes and branch
    ///
    /// This instruction compares the unsigned byte values in two registers, and conditionally branches to a label at a PC-relative
    /// offset if the second value is lower than the first. This instruction provides a hint that this is not a subroutine call or return.
    ///
    /// This instruction does not affect the condition flags.
    pub fn cbb_lo(&mut self, m: Register, H: bool, imm9: u16, t: Register) {
        self.cbb_cc(0b010, m, H, imm9, t)
    }

    /// **CBBLS**: Compare unsigned lower than or same as bytes and branch
    ///
    /// This instruction compares the unsigned byte values in two registers, and conditionally branches to a label at a PC-relative
    /// offset if the second value is lower than or the same as the first. This instruction provides a hint that this is not a subroutine
    /// call or return. This instruction does not affect the condition flags.
    pub fn cbb_ls(&mut self, m: Register, H: bool, imm9: u16, t: Register) {
        self.cbb_cc(0b011, m, H, imm9, t)
    }

    /// **CBBLT**: Compare signed less than bytes and branch
    ///
    /// This instruction compares the signed byte values in two registers, and conditionally branches to a label at a PC-relative
    /// offset if the second value is less than the first. This instruction provides a hint that this is not a subroutine call or return. This
    /// instruction does not affect the condition flags.
    pub fn cbb_lt(&mut self, m: Register, H: bool, imm9: u16, t: Register) {
        self.cbb_cc(0b000, m, H, imm9, t)
    }

    /// **CB&lt;cc&gt; (immediate)**: Compare register with immediate and branch
    ///
    /// This instruction compares the value in a register with an immediate, and conditionally branches to a label at a PC-relative
    /// offset if the comparison is true. This instruction provides a hint that this is not a subroutine call or return. This instruction
    /// does not affect the condition flags.
    pub fn cb_cc_imm(&mut self, sf: bool, cc: u8, imm6: u8, imm9: u16, t: Register) {
        let offset = shared::sint::<11>((imm9 << 2) as u64);

        let (op, unsigned) = match cc {
            0b000 => (CmpOp::GT, false),
            0b001 => (CmpOp::LT, false),
            0b010 => (CmpOp::GT, true),
            0b011 => (CmpOp::LT, true),
            0b110 => (CmpOp::EQ, true),
            0b111 => (CmpOp::NE, true),
            _ => unreachable!(),
        };

        let operand1 = self._get_op(t, sf);

        let cond = if unsigned {
            let value1 = operand1;
            let value2 = imm6 as u64;

            match op {
                CmpOp::EQ => value1 == value2,
                CmpOp::NE => value1 != value2,
                CmpOp::GT => value1 > value2,
                CmpOp::LT => value1 < value2,
                _ => unreachable!(),
            }
        } else {
            let value1 = operand1 as i64;
            let value2 = imm6 as i64;

            match op {
                CmpOp::EQ => value1 == value2,
                CmpOp::NE => value1 != value2,
                CmpOp::GT => value1 > value2,
                CmpOp::LT => value1 < value2,
                _ => unreachable!(),
            }
        };

        if cond {
            self.branch_to::<64>(((self.pc as i64) + offset) as u64, BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CB&lt;cc&gt; (register)**: Compare registers and branch
    ///
    /// This instruction compares the values in two registers, and conditionally branches to a label at a PC-relative offset if the
    /// condition is true. This instruction provides a hint that this is not a subroutine call or return. This instruction does not affect
    /// the condition flags.
    pub fn cb_cc_reg(&mut self, sf: bool, cc: u8, m: Register, imm9: u16, t: Register) {
        let offset = shared::sint::<11>((imm9 << 2) as u64);

        let (op, unsigned) = match cc {
            0b000 => (CmpOp::GT, false),
            0b001 => (CmpOp::GE, false),
            0b010 => (CmpOp::GT, true),
            0b011 => (CmpOp::GE, true),
            0b110 => (CmpOp::EQ, true),
            0b111 => (CmpOp::NE, true),
            _ => unreachable!(),
        };

        let operand1 = self._get_op(t, sf);
        let operand2 = self._get_op(m, sf);

        let cond = if unsigned {
            let value1 = operand1;
            let value2 = operand2;

            match op {
                CmpOp::EQ => value1 == value2,
                CmpOp::NE => value1 != value2,
                CmpOp::GT => value1 > value2,
                CmpOp::LT => value1 < value2,
                _ => unreachable!(),
            }
        } else {
            let value1 = operand1 as i64;
            let value2 = operand2 as i64;

            match op {
                CmpOp::EQ => value1 == value2,
                CmpOp::NE => value1 != value2,
                CmpOp::GT => value1 > value2,
                CmpOp::LT => value1 < value2,
                _ => unreachable!(),
            }
        };

        if cond {
            self.branch_to::<64>(((self.pc as i64) + offset) as u64, BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CBH&lt;cc&gt;**: Compare halfwords and branch
    /// This instruction compares the halfword values in two registers, and conditionally branches to a label at a PC-relative offset
    /// if the condition is true. This instruction provides a hint that this is not a subroutine call or return. This instruction does not
    /// affect the condition flags.
    #[allow(unused_variables)]
    pub fn cbh_cc(&mut self, cc: u8, m: Register, imm9: u16, t: Register) {
        todo!()
    }
}
