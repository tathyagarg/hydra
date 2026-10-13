use crate::cpu::{
    common::nth_bit,
    regs::{RegisterField, RegisterFieldRange},
};

#[allow(non_camel_case_types)]
pub enum TTBR0_128_EL1Field {
    BADDR_50_43 = -1, // 87-80
    ASID = -2,        // 63-48
    BADDR_42_0 = -3,  // 47-5
    SKL = -4,         // 2-1
    CnP = 0,
}

impl RegisterField<u128> for TTBR0_128_EL1Field {
    fn fetch_bit(self, v: u128) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u128, u8> for TTBR0_128_EL1Field {
    fn fetch_range(self, v: u128) -> u8 {
        if let Self::BADDR_50_43 = self {
            return ((v >> 80) & 0xFF) as u8;
        }

        if let Self::SKL = self {
            return ((v >> 1) & 0b11) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u128, u16> for TTBR0_128_EL1Field {
    fn fetch_range(self, v: u128) -> u16 {
        if let Self::ASID = self {
            return ((v >> 48) & 0xFFFF) as u16;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u128, u64> for TTBR0_128_EL1Field {
    fn fetch_range(self, v: u128) -> u64 {
        if let Self::BADDR_42_0 = self {
            return ((v >> 5) & 0x7ffffffffff) as u64;
        }

        unreachable!()
    }
}

#[allow(non_camel_case_types)]
pub enum TTBR0_64_EL1Field {
    ASID = -2,       // 63-48
    BADDR_47_1 = -3, // 47-1
    CnP = 0,
}

impl RegisterField<u64> for TTBR0_64_EL1Field {
    fn fetch_bit(self, v: u64) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u64, u16> for TTBR0_64_EL1Field {
    fn fetch_range(self, v: u64) -> u16 {
        if let Self::ASID = self {
            return ((v >> 48) & 0xFFFF) as u16;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u64> for TTBR0_64_EL1Field {
    fn fetch_range(self, v: u64) -> u64 {
        if let Self::BADDR_47_1 = self {
            return ((v >> 1) & 0x7fffffffffff) as u64;
        }

        unreachable!()
    }
}

#[allow(non_camel_case_types)]
pub enum TTBR0_128_EL2Field {
    BADDR_50_43 = -1, // 87-80
    ASID = -2,        // 63-48
    BADDR_42_0 = -3,  // 47-5
    SKL = -4,         // 2-1
    CnP = 0,
}

impl RegisterField<u128> for TTBR0_128_EL2Field {
    fn fetch_bit(self, v: u128) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u128, u8> for TTBR0_128_EL2Field {
    fn fetch_range(self, v: u128) -> u8 {
        if let Self::BADDR_50_43 = self {
            return ((v >> 80) & 0xFF) as u8;
        }

        if let Self::SKL = self {
            return ((v >> 1) & 0b11) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u128, u16> for TTBR0_128_EL2Field {
    fn fetch_range(self, v: u128) -> u16 {
        if let Self::ASID = self {
            return ((v >> 48) & 0xFFFF) as u16;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u128, u64> for TTBR0_128_EL2Field {
    fn fetch_range(self, v: u128) -> u64 {
        if let Self::BADDR_42_0 = self {
            return ((v >> 5) & 0x7ffffffffff) as u64;
        }

        unreachable!()
    }
}

#[allow(non_camel_case_types)]
pub enum TTBR0_64_EL2Field {
    ASID = -2,       // 63-48
    BADDR_47_1 = -3, // 47-1
    CnP = 0,
}

impl RegisterField<u64> for TTBR0_64_EL2Field {
    fn fetch_bit(self, v: u64) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u64, u16> for TTBR0_64_EL2Field {
    fn fetch_range(self, v: u64) -> u16 {
        if let Self::ASID = self {
            return ((v >> 48) & 0xFFFF) as u16;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u64> for TTBR0_64_EL2Field {
    fn fetch_range(self, v: u64) -> u64 {
        if let Self::BADDR_47_1 = self {
            return ((v >> 1) & 0x7fffffffffff) as u64;
        }

        unreachable!()
    }
}

#[allow(non_camel_case_types)]
pub enum TTBR0_128_EL3Field {
    BADDR = -1, // 55-5
    SKL = -2,   // 2-1
    CnP = 0,
}

impl RegisterField<u64> for TTBR0_128_EL3Field {
    fn fetch_bit(self, v: u64) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u64, u8> for TTBR0_128_EL3Field {
    fn fetch_range(self, v: u64) -> u8 {
        if let Self::SKL = self {
            return ((v >> 1) & 0b11) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u64> for TTBR0_128_EL3Field {
    fn fetch_range(self, v: u64) -> u64 {
        if let Self::BADDR = self {
            return ((v >> 5) & 0x7ffffffffffff) as u64;
        }

        unreachable!()
    }
}

#[allow(non_camel_case_types)]
pub enum TTBR0_64_EL3Field {
    BADDR = -1, // 47-1
    CnP = 0,
}

impl RegisterField<u64> for TTBR0_64_EL3Field {
    fn fetch_bit(self, v: u64) -> bool {
        nth_bit(v, self as usize)
    }
}

impl RegisterFieldRange<u64, u64> for TTBR0_64_EL3Field {
    fn fetch_range(self, v: u64) -> u64 {
        if let Self::BADDR = self {
            return ((v >> 1) & 0x7fffffffffff) as u64;
        }

        unreachable!()
    }
}
