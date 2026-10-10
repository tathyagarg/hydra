pub struct Memory {
    data: Vec<u8>,
}

pub trait ReadableInt: Sized {
    fn from_bytes(slice: &[u8]) -> Option<Self>;
}

impl ReadableInt for u8 {
    fn from_bytes(slice: &[u8]) -> Option<Self> {
        Some(*slice.get(0)?)
    }
}

impl ReadableInt for u16 {
    fn from_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(u16::from_le_bytes(bytes))
    }
}

impl ReadableInt for u32 {
    fn from_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    }
}

impl ReadableInt for u64 {
    fn from_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    }
}

impl Memory {
    pub fn read<T: ReadableInt>(&self, offset: usize) -> Option<T> {
        let slice = self.data.get(offset..)?;
        T::from_bytes(slice)
    }
}
