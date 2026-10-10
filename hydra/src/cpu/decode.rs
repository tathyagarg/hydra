use crate::cpu::Register;

#[derive(Debug)]
pub enum Instruction {
    Abs {
        sf: bool,
        rn: Register,
        rd: Register,
    },
    Adc {
        sf: bool,
        rm: Register,
        rn: Register,
        rd: Register,
    },
    Adcs {
        sf: bool,
        rm: Register,
        rn: Register,
        rd: Register,
    },
    AddEr {
        sf: bool,
        rm: Register,
        option: u8,
        imm3: u8,
        rn: Register,
        rd: Register,
    },
    AddImm {
        sf: bool,
        sh: bool,
        imm12: u16,
        rn: Register,
        rd: Register,
    },
    AddSr {
        sf: bool,
        shift: u8,
        rm: Register,
        imm6: u8,
        rn: Register,
        rd: Register,
    },
    Movz {
        sf: bool,
        rd: Register,
        imm16: u16,
        hw: u8,
    },
    SubEr {
        sf: bool,
        rm: Register,
        option: u8,
        imm3: u8,
        rn: Register,
        rd: Register,
    },
    SubImm {
        sf: bool,
        sh: bool,
        imm12: u16,
        rn: Register,
        rd: Register,
    },
    SubSr {
        sf: bool,
        shift: u8,
        rm: Register,
        imm6: u8,
        rn: Register,
        rd: Register,
    },
}

const ABS_M: u32 = 0b01111111111111111111110000000000;
const ABS_V: u32 = 0b01011010110000000010000000000000;

const ADC_M: u32 = 0b01111111111000001111110000000000;
const ADC_V: u32 = 0b00011010000000000000000000000000;

const ADCS_M: u32 = 0b01111111111000001111110000000000;
const ADCS_V: u32 = 0b00111010000000000000000000000000;

const ADD_ER_M: u32 = 0b01111111111000000000000000000000;
const ADD_ER_V: u32 = 0b00001011001000000000000000000000;

const ADD_IMM_M: u32 = 0b01111111100000000000000000000000;
const ADD_IMM_V: u32 = 0b00010001000000000000000000000000;

const ADD_SR_M: u32 = 0b01111111001000000000000000000000;
const ADD_SR_V: u32 = 0b00001011000000000000000000000000;

const MOVZ_M: u32 = 0b01111111100000000000000000000000;
const MOVZ_V: u32 = 0b01010010100000000000000000000000;

const SUB_ER_M: u32 = 0b01111111111000000000000000000000;
const SUB_ER_V: u32 = 0b01001011001000000000000000000000;

const SUB_IMM_M: u32 = 0b01111111100000000000000000000000;
const SUB_IMM_V: u32 = 0b01010001000000000000000000000000;

const SUB_SR_M: u32 = 0b01111111001000000000000000000000;
const SUB_SR_V: u32 = 0b01001011000000000000000000000000;

fn sf_decode(word: u32) -> bool {
    (word >> 31) & 1 == 1
}

fn reg_decode(word: u32, pos: usize) -> Register {
    ((word >> pos) & 0b11111) as Register
}

fn decode_abs(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::Abs { sf, rn, rd }
}

fn decode_adc(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let rm = reg_decode(word, 16);
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::Adc { sf, rm, rn, rd }
}

fn decode_adcs(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let rm = reg_decode(word, 16);
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::Adcs { sf, rm, rn, rd }
}

fn decode_add_er(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let rm = reg_decode(word, 16);
    let option = ((word >> 13) & 0b111) as Register;
    let imm3 = ((word >> 13) & 0b111) as Register;
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::AddEr {
        sf,
        rm,
        option,
        imm3,
        rn,
        rd,
    }
}

fn decode_add_imm(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let sh = ((word >> 22) & 1) == 1;
    let imm12 = ((word >> 10) & 0xFFF) as u16;
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::AddImm {
        sf,
        sh,
        imm12,
        rn,
        rd,
    }
}

fn decode_add_sr(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let shift = ((word >> 22) & 0b11) as Register;
    let rm = reg_decode(word, 16);
    let imm6 = ((word >> 10) & 0b111111) as Register;
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::AddSr {
        sf,
        shift,
        rm,
        imm6,
        rn,
        rd,
    }
}

fn decode_movz(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let rd = reg_decode(word, 0);
    let imm16 = ((word >> 5) & 0xFFFF) as u16;
    let hw = ((word >> 21) & 0b11) as u8;

    Instruction::Movz { sf, rd, imm16, hw }
}

fn decode_sub_er(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let rm = reg_decode(word, 16);
    let option = ((word >> 13) & 0b111) as Register;
    let imm3 = ((word >> 13) & 0b111) as Register;
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::SubEr {
        sf,
        rm,
        option,
        imm3,
        rn,
        rd,
    }
}

fn decode_sub_imm(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let sh = ((word >> 22) & 1) == 1;
    let imm12 = ((word >> 10) & 0xFFF) as u16;
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::SubImm {
        sf,
        sh,
        imm12,
        rn,
        rd,
    }
}

fn decode_sub_sr(word: u32) -> Instruction {
    let sf = sf_decode(word);
    let shift = ((word >> 22) & 0b11) as Register;
    let rm = reg_decode(word, 16);
    let imm6 = ((word >> 10) & 0b111111) as Register;
    let rn = reg_decode(word, 5);
    let rd = reg_decode(word, 0);

    Instruction::SubSr {
        sf,
        shift,
        rm,
        imm6,
        rn,
        rd,
    }
}

const IMPLEMENTED_INSTRUCTION_MAP: [(u32, u32, fn(u32) -> Instruction); 10] = [
    (ABS_M, ABS_V, decode_abs),
    (ADC_M, ADC_V, decode_adc),
    (ADCS_M, ADCS_V, decode_adcs),
    (ADD_ER_M, ADD_ER_V, decode_add_er),
    (ADD_IMM_M, ADD_IMM_V, decode_add_imm),
    (ADD_SR_M, ADD_SR_V, decode_add_sr),
    (MOVZ_M, MOVZ_V, decode_movz),
    (SUB_ER_M, SUB_ER_V, decode_sub_er),
    (SUB_IMM_M, SUB_IMM_V, decode_sub_imm),
    (SUB_SR_M, SUB_SR_V, decode_sub_sr),
];

/// Decode an instruction word into respective instruction structure that contains serialized
/// fields that can later be used to call the respective command functionality.
///
/// ```
/// # use hydra::cpu::decode::{decode, Instruction};
///
/// let input = 0b11011010110000000010001010000101;
/// let res = decode(input);
///
/// assert!(res.is_some());
/// if let Some(Instruction::Abs { sf, rn, rd }) = res {
///     assert!(sf);
///     assert_eq!(rn, 20);
///     assert_eq!(rd, 5);
/// }
/// ```
pub fn decode(word: u32) -> Option<Instruction> {
    for (mask, value, f) in IMPLEMENTED_INSTRUCTION_MAP {
        if word & mask == value {
            return Some(f(word));
        }
    }

    None
}
