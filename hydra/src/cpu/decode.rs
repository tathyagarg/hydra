pub enum Instruction {
    Abs {
        sf: bool,
        rn: u8,
        rd: u8,
    },
    Adc {
        sf: bool,
        rm: u8,
        rn: u8,
        rd: u8,
    },
    Adcs {
        sf: bool,
        rm: u8,
        rn: u8,
        rd: u8,
    },
    AddEr {
        sf: bool,
        rm: u8,
        option: u8,
        imm3: u8,
        rn: u8,
        rd: u8,
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

fn sf_decode(word: u32) -> bool {
    (word >> 31) & 1 == 1
}

fn reg_decode(word: u32, pos: usize) -> u8 {
    ((word >> pos) & 0b11111) as u8
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
    let option = ((word >> 13) & 0b111) as u8;
    let imm3 = ((word >> 13) & 0b111) as u8;
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

const IMPLEMENTED_INSTRUCTION_MAP: [(u32, u32, fn(u32) -> Instruction); 4] = [
    (ABS_M, ABS_V, decode_abs),
    (ADC_M, ADC_V, decode_adc),
    (ADCS_M, ADCS_V, decode_adcs),
    (ADD_ER_M, ADD_ER_V, decode_add_er),
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
