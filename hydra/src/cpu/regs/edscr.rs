use crate::cpu::regs::RegisterFieldRange;

pub enum EDSCRField {
    TFO = 31,
    RXFull = 30,
    TXFull = 29,
    ITO = 28,
    RXO = 27,
    TXU = 26,
    PipeAdv = 25,
    ITE = 24,
    INTdis = -1, // 23-22
    TDA = 21,
    MA = 20,
    SC2 = 19,
    NS = 18,
    SDD = 16,
    NSE = 15,
    HDE = 14,
    RW = -2, // 13-10
    EL = -3, // 9-8
    A = 7,
    ERR = 6,
    STATUS = -4, // 5-0
}

impl RegisterFieldRange<u32, u8> for EDSCRField {
    fn fetch_range(self, edscr_v: u32) -> u8 {
        (match self {
            Self::INTdis => (edscr_v >> 22) & 0b11,
            Self::RW => (edscr_v >> 10) & 0b1111,
            Self::EL => (edscr_v >> 8) & 0b11,
            Self::STATUS => edscr_v & 0b1111,
            _ => unreachable!(),
        }) as u8
    }
}
