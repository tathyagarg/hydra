use crate::cpu::common::nth_bit;

#[derive(PartialEq, Clone)]
pub struct Bits<const N: usize> {
    // bytes.len() == N / 8
    bytes: Vec<u8>,
}

impl<const N: usize> Bits<N> {
    pub fn zero() -> Bits<N> {
        Bits { bytes: vec![0] }
    }

    pub fn reverse(&self) -> Self {
        Self {
            bytes: self.bytes.iter().rev().copied().collect(),
        }
    }

    pub fn nth_bit(&self, n: usize) -> bool {
        let byte_index = n / 8;
        if byte_index > self.bytes.len() {
            panic!("Byte index out of range");
        }

        nth_bit(self.bytes[byte_index], n % 8)
    }

    pub fn get_bits(&self, x: usize, y: usize) -> u64 {
        let mut result = 0u64;

        for bit in x..=y {
            let byte_index = bit / 8;
            let bit_index = bit % 8;
            let value = (self.bytes[byte_index] >> bit_index) & 1;

            result |= (value as u64) << (bit - x);
        }

        result
    }

    pub fn set_bit(&mut self, n: usize, v: bool) {
        let byte_index = n / 8;
        let bit_index = n % 8;
        let mask = 1u8 << bit_index;

        if v {
            self.bytes[byte_index] |= mask;
        } else {
            self.bytes[byte_index] &= !mask;
        }
    }

    pub fn compile_u128(&self) -> u128 {
        let mut result = 0u128;
        for byte in &self.bytes {
            result |= *byte as u128;
            result <<= 8;
        }

        result
    }

    pub fn extend_to<const S: usize>(self) -> Bits<S> {
        assert!(S > N);

        Bits {
            bytes: [vec![0; (S - N) / 8], self.bytes].concat(),
        }
    }

    pub fn shrink_to<const S: usize>(self) -> Bits<S> {
        assert!(S < N);

        Bits {
            bytes: self.bytes[(N - S) / 8..0].to_vec(),
        }
    }
}
