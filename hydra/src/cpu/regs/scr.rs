use crate::cpu::common::nth_bit;

pub enum SCRField {
    TERR = 15,
    TWE = 13,
    TWI = 12,
    SIF = 9,
    HCE = 8,
    SCD = 7,
    #[allow(non_camel_case_types)]
    nET = 6,
    AW = 5,
    FW = 4,
    EA = 3,
    FIQ = 2,
    IRQ = 1,
    NS = 0,
}

impl SCRField {
    pub fn fetch_bit(self, scr_v: u32) -> bool {
        nth_bit::<u32>(scr_v, self as usize)
    }
}
