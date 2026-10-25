use crate::cpu::{common::nth_bit, regs::RegisterField};

pub enum HCR2Field {
    TTLBIS = 22,
    TOCU = 20,
    TICAB = 18,
    TID4 = 17,
    TEA = 5,
    TERR = 4,
    ID = 1,
    CD = 0,
}

impl RegisterField<u32> for HCR2Field {
    fn fetch_bit(self, v: u32) -> bool {
        nth_bit(v, self as usize)
    }
}
