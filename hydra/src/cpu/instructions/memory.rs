use crate::cpu::{
    Cpu, DecodeType, REG_SP, Register,
    common::{nth_bit, ones},
    constraint::{Constraint, Unpredictable},
    fault::MemOp,
    shared::{self, EL0},
};

impl Cpu {
    #[allow(unused_variables, unused_assignments)]
    pub fn ldr_imm_post(&mut self, size: u8, imm9: u16, n: Register, t: Register) {
        let wback = true;

        let postindex = true;
        let scale = size;
        let offset = shared::sint::<9, u16>(imm9);

        let (wback, wb_unknown) = self._ldr_imm_decode(wback, n, t);

        let privileged = self.pstate.EL() != EL0;
        let accdesc =
            self.create_accdesc_gpr_rt(MemOp::LOAD, false, privileged, wback || n != 31, Some(t));

        let mut address = if n == 31 {
            todo!("Check spa alignment");
        } else {
            self._get_op(n, true)
        };

        if !postindex {
            address = self.address_add(address, offset as u64, &accdesc);
        }

        todo!("too much work")
    }

    fn _ldr_imm_decode(&self, mut wback: bool, n: Register, t: Register) -> (bool, bool) {
        let mut wb_unknown = false;

        if wback && n == t && n != REG_SP {
            let c = Unpredictable::WBOVERLAPLD.constrain();
            assert!(
                [
                    Constraint::WBSUPPRESS,
                    Constraint::UNKNOWN,
                    Constraint::UNDEF,
                    Constraint::NOP
                ]
                .contains(&c)
            );

            match c {
                Constraint::WBSUPPRESS => wback = false,
                Constraint::UNKNOWN => wb_unknown = true,
                Constraint::UNDEF => self.end_of_decode(DecodeType::UNDEF),
                Constraint::NOP => self.end_of_decode(DecodeType::NOP),
                _ => unreachable!(),
            }
        }

        (wback, wb_unknown)
    }

    /// **MOVK**: Move wide with keep
    ///
    /// This instruction moves an optionally-shifted 16-bit immediate value into a register, keeping other bits unchanged.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.movk(true, 2, 0xFFFF, 0);
    /// assert_eq!(cpu.__dbg_get_x(0), 281470681743360);
    /// ```
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.__dbg_set_x(0, 6536370415110711463);
    /// cpu.movk(true, 2, 0xFFFF, 0);
    /// assert_eq!(cpu.__dbg_get_x(0), 6536411908789758119);
    /// ```
    pub fn movk(&mut self, sf: bool, hw: u8, imm16: u16, d: Register) {
        if !sf && nth_bit(hw, 1) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let pos = hw << 4;
        let mask = ones(16) << pos;

        let mut result = self._get_op(d, sf);
        result = (result & !mask) | ((imm16 as u64) << pos);

        self._set_op(d, result, sf);
    }

    /// **MOVN**: Move wide with NOT
    ///
    /// This instruction moves the inverse of an optionally-shifted 16-bit immediate value to a register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.movn(true, 2, 0xFFFF, 0);
    /// assert_eq!(cpu.__dbg_get_x(0), 18446462603027808255);
    /// ```
    pub fn movn(&mut self, sf: bool, hw: u8, imm16: u16, d: Register) {
        if !sf && nth_bit(hw, 1) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let pos = hw << 4;
        let result = (imm16 as u64) << pos;

        self._set_op(d, !result, sf);
    }

    /// **MOVZ**: Move wide with zero
    ///
    /// This instruction moves an optionally-shifted 16-bit immediate value to a register.
    ///
    /// ```
    /// # use hydra::cpu::Cpu;
    /// # let mut cpu = Cpu::default();
    /// cpu.movz(true, 2, 0xFFFF, 0);
    /// assert_eq!(cpu.__dbg_get_x(0), 0x0000FFFF00000000);
    /// ```
    pub fn movz(&mut self, sf: bool, hw: u8, imm16: u16, d: Register) {
        if !sf && nth_bit(hw, 1) {
            self.end_of_decode(DecodeType::UNDEF);
        }

        let pos = hw << 4;
        let result = (imm16 as u64) << pos;

        self._set_op(d, result, sf);
    }
}
