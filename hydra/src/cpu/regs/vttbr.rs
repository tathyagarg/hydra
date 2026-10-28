use crate::cpu::{common::get_bits, regs::RegisterFieldRange};

pub enum VTTBRField {
    VMID = -1,  // 55-48
    BADDR = -2, // 47-1
    CnP = 0,
}

impl RegisterFieldRange<u64, u8> for VTTBRField {
    fn fetch_range(self, v: u64) -> u8 {
        if let Self::VMID = self {
            return get_bits(v, 55, 48) as u8;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u64> for VTTBRField {
    fn fetch_range(self, v: u64) -> u64 {
        if let Self::BADDR = self {
            return get_bits(v, 47, 1) as u64;
        }

        unreachable!()
    }
}

#[allow(non_camel_case_types)]
pub enum VMSAv9_128_VTTBR_EL2Field {
    BADDR = -1,
    VMID = -2, // 63-48
    SKL = -3,  // 2-1
    CnP = 0,
}

impl RegisterFieldRange<u128, u64> for VMSAv9_128_VTTBR_EL2Field {
    fn fetch_range(self, v: u128) -> u64 {
        if let Self::BADDR = self {
            return (get_bits(v, 87, 80) << 43) as u64 | get_bits(v, 47, 5) as u64;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u128, u16> for VMSAv9_128_VTTBR_EL2Field {
    fn fetch_range(self, v: u128) -> u16 {
        if let Self::VMID = self {
            return get_bits(v, 63, 48) as u16;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u128, u8> for VMSAv9_128_VTTBR_EL2Field {
    fn fetch_range(self, v: u128) -> u8 {
        if let Self::SKL = self {
            return get_bits(v, 2, 1) as u8;
        }

        unreachable!()
    }
}

#[allow(non_camel_case_types)]
pub enum VMSAv8_64_VTTBR_EL2Field {
    BADDR = -1, // 47-1
    VMID = -2,  // 63-48
    CnP = 0,
}

impl RegisterFieldRange<u64, u64> for VMSAv8_64_VTTBR_EL2Field {
    fn fetch_range(self, v: u64) -> u64 {
        if let Self::BADDR = self {
            return get_bits(v, 47, 1) as u64;
        }

        unreachable!()
    }
}

impl RegisterFieldRange<u64, u16> for VMSAv8_64_VTTBR_EL2Field {
    fn fetch_range(self, v: u64) -> u16 {
        if let Self::VMID = self {
            return get_bits(v, 63, 48) as u16;
        }

        unreachable!()
    }
}
