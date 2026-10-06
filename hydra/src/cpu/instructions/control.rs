use crate::cpu::{BranchType, CmpOp, Cpu, DecodeType, Feature, Register, common::*, shared};

impl Cpu {
    /// **B.cond**: Branch conditionally
    ///
    /// This instruction branches conditionally to a label at a PC-relative offset. This instruction provides a hint that this is not a
    /// subroutine call or return.
    pub fn b_cond(&mut self, imm19: u32, cond: u8) {
        let offset = shared::sint::<21, u32>(imm19 << 2) as u64;
        if self.condition_holds(cond) {
            self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **B**: Branch
    ///
    /// This instruction branches unconditionally to a label at a PC-relative offset. This instruction provides a hint that this is not a
    /// subroutine call or return.
    pub fn b(&mut self, imm26: u32) {
        let offset = shared::sint::<28, u32>(imm26 << 2) as u64;
        self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, false);
    }

    /// **BC.cond**: Branch consistent conditionally
    ///
    /// This instruction branches conditionally to a label at a PC-relative offset, with a hint that this branch is very unlikely to
    /// change direction. This instruction provides a hint that this is not a subroutine call or return.
    pub fn bc_cond(&mut self, imm19: u32, cond: u8) {
        if !Cpu::is_feature_implemented(Feature::HBC) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let offset = shared::sint::<21, u32>(imm19 << 2) as u64;

        if self.condition_holds(cond) {
            self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CBB&lt;cc&gt;**: Compare bytes and branch
    ///
    /// This instruction compares the byte values in two registers, and conditionally branches to a label at a PC-relative offset if the
    /// condition is true. This instruction provides a hint that this is not a subroutine call or return. This instruction does not affect
    /// the condition flags.
    pub fn cbb_cc(&mut self, cc: u8, m: Register, H: bool, imm9: u16, t: Register) {
        if !Cpu::is_feature_implemented(Feature::CMPBR) {
            self.end_of_decode(DecodeType::UNDEF);
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

        let offset = shared::sint::<11, u16>(imm9 << 2) as u64;

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
            self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CB&lt;cc&gt; (immediate)**: Compare register with immediate and branch
    ///
    /// This instruction compares the value in a register with an immediate, and conditionally branches to a label at a PC-relative
    /// offset if the comparison is true. This instruction provides a hint that this is not a subroutine call or return. This instruction
    /// does not affect the condition flags.
    pub fn cb_cc_imm(&mut self, sf: bool, cc: u8, imm6: u8, imm9: u16, t: Register) {
        if !Cpu::is_feature_implemented(Feature::CMPBR) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let offset = shared::sint::<11, u16>(imm9 << 2) as u64;

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
            self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, true);
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
        if !Cpu::is_feature_implemented(Feature::CMPBR) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let offset = shared::sint::<11, u16>(imm9 << 2) as u64;

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
            self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CBH&lt;cc&gt;**: Compare halfwords and branch
    ///
    /// This instruction compares the halfword values in two registers, and conditionally branches to a label at a PC-relative offset
    /// if the condition is true. This instruction provides a hint that this is not a subroutine call or return. This instruction does not
    /// affect the condition flags.
    #[allow(unused_variables)]
    pub fn cbh_cc(&mut self, cc: u8, m: Register, imm9: u16, t: Register) {
        todo!()
    }

    /// **CBNZ**: Compare and branch on nonzero
    ///
    /// This instruction compares the value in a register with zero, and conditionally branches to a label at a PC-relative offset if the
    /// comparison is not equal. This instruction provides a hint that this is not a subroutine call or return. This instruction does not
    /// affect the condition flags.
    pub fn cbnz(&mut self, sf: bool, imm19: u32, t: Register) {
        let offset = shared::sint::<21, u32>(imm19 << 2) as u64;

        let operand1 = self._get_op(t, sf);
        if operand1 != 0 {
            self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CBZ**: Compare and branch on zero
    ///
    /// This instruction compares the value in a register with zero, and conditionally branches to a label at a PC-relative offset if the
    /// comparison is equal. This instruction provides a hint that this is not a subroutine call or return. This instruction does not
    /// affect condition flags.
    pub fn cbz(&mut self, sf: bool, imm19: u32, t: Register) {
        let offset = shared::sint::<21, u32>(imm19 << 2) as u64;

        let operand1 = self._get_op(t, sf);
        if operand1 == 0 {
            self.branch_to::<64>(self.get_PC64().wrapping_add(offset), BranchType::DIR, true);
        } else {
            self.branch_not_taken(BranchType::DIR, true);
        }
    }

    /// **CCMN (immediate)**: Conditional compare negative (immediate)
    ///
    /// This instruction sets the value of the condition flags to the result of the comparison of a register value and a negated
    /// immediate value if the condition is TRUE, and an immediate value otherwise.
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType, common::NZCV};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFFFF_FFFF);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(true);  // make the condition hold
    /// cpu.ccmn_imm(false, 1, 0b0000, 0, 0b1111);
    /// assert_eq!(cpu.__dbg_get_pstate().nzcv(), NZCV::init(false, true, true, false));
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::{Cpu, ExtendType, common::NZCV};
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 0xFFFF_FFFF);
    /// cpu.__dbg_get_pstate().__dbg_set_Z(false);  // make the condition not hold
    /// cpu.ccmn_imm(false, 1, 0b0000, 0, 0b1111);
    /// assert_eq!(cpu.__dbg_get_pstate().nzcv(), NZCV::init(true, true, true, true));
    /// ```
    pub fn ccmn_imm(&mut self, sf: bool, imm5: u8, cond: u8, n: Register, nzcv: u8) {
        let mut flags = NZCV::from(nzcv);

        if self.condition_holds(cond) {
            let operand1 = self._get_op(n, sf);
            let operand2 = imm5 as u64;

            let (_, rflags) = if sf {
                shared::add_with_carry::<64>(operand1, operand2, false)
            } else {
                shared::add_with_carry::<32>(operand1, operand2, false)
            };

            flags = rflags;
        }

        self.pstate.set_nzcv(&flags);
    }

    /// **CCMN (register)**: Conditional compare negative (register)
    ///
    /// This instruction sets the value of the condition flags to the result of the comparison of a register value and the inverse of
    /// another register value if the condition is TRUE, and an immediate value otherwise.
    pub fn ccmn_reg(&mut self, sf: bool, m: Register, cond: u8, n: Register, nzcv: NZCV) {
        let mut flags = nzcv;

        if self.condition_holds(cond) {
            let operand1 = self._get_op(n, sf);
            let operand2 = self._get_op(m, sf);

            let (_, rflags) = if sf {
                shared::add_with_carry::<64>(operand1, operand2, false)
            } else {
                shared::add_with_carry::<32>(operand1, operand2, false)
            };

            flags = rflags;
        }

        self.pstate.set_nzcv(&flags);
    }

    /// **CCMP (immediate)**: Conditional compare (immediate)
    ///
    /// This instruction sets the value of the condition flags to the result of the comparison of a register value and an immediate
    /// value if the condition is TRUE, and an immediate value otherwise.
    pub fn ccmp_imm(&mut self, sf: bool, imm5: u8, cond: u8, n: Register, nzcv: u8) {
        let mut flags = NZCV::from(nzcv);

        if self.condition_holds(cond) {
            let operand1 = self._get_op(n, sf);
            let operand2 = imm5 as u64;

            let (_, rflags) = if sf {
                shared::add_with_carry::<64>(operand1, !operand2, false)
            } else {
                shared::add_with_carry::<32>(operand1, !operand2, false)
            };

            flags = rflags;
        }

        self.pstate.set_nzcv(&flags);
    }

    /// **CCMP (register)**: Conditional compare (register)
    ///
    /// This instruction sets the value of the condition flags to the result of the comparison of two registers if the condition is TRUE,
    /// and an immediate value otherwise.
    pub fn ccmp_reg(&mut self, sf: bool, m: Register, cond: u8, n: Register, nzcv: NZCV) {
        let mut flags = nzcv;

        if self.condition_holds(cond) {
            let operand1 = self._get_op(n, sf);
            let operand2 = self._get_op(m, sf);

            let (_, rflags) = if sf {
                shared::add_with_carry::<64>(operand1, !operand2, false)
            } else {
                shared::add_with_carry::<32>(operand1, !operand2, false)
            };

            flags = rflags;
        }

        self.pstate.set_nzcv(&flags);
    }
}
