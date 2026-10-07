use crate::cpu::{
    Cpu, DecodeType, REG_SP, Register,
    constraint::{Constraint, Unpredictable},
    fault::MemOp,
    shared::{self, EL0},
};

impl Cpu {
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
            self.sp
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
}
