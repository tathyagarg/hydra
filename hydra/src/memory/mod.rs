pub enum Endian {
    Big,
    Little,
}

pub struct Memory {
    data: Vec<u8>,
    endian: Endian,
}

pub trait ReadableInt: Sized {
    fn from_le_bytes(slice: &[u8]) -> Option<Self>;
    fn from_be_bytes(slice: &[u8]) -> Option<Self>;
}

impl ReadableInt for u8 {
    fn from_le_bytes(slice: &[u8]) -> Option<Self> {
        Some(*slice.get(0)?)
    }

    fn from_be_bytes(slice: &[u8]) -> Option<Self> {
        Some(*slice.get(0)?)
    }
}

impl ReadableInt for u16 {
    fn from_le_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(Self::from_le_bytes(bytes))
    }

    fn from_be_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(Self::from_be_bytes(bytes))
    }
}

impl ReadableInt for u32 {
    fn from_le_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(Self::from_le_bytes(bytes))
    }

    fn from_be_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(Self::from_be_bytes(bytes))
    }
}

impl ReadableInt for u64 {
    fn from_le_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(Self::from_le_bytes(bytes))
    }

    fn from_be_bytes(slice: &[u8]) -> Option<Self> {
        let bytes = slice.get(..std::mem::size_of::<Self>())?.try_into().ok()?;
        Some(Self::from_be_bytes(bytes))
    }
}

impl From<Vec<u8>> for Memory {
    fn from(data: Vec<u8>) -> Self {
        Self {
            data,
            endian: Endian::Big,
        }
    }
}

impl Memory {
    pub fn init(data: Vec<u8>, endian: Endian) -> Self {
        Self { data, endian }
    }

    pub fn read<T: ReadableInt>(&self, offset: usize) -> Option<T> {
        let slice = self.data.get(offset..)?;
        match self.endian {
            Endian::Big => T::from_be_bytes(slice),
            Endian::Little => T::from_le_bytes(slice),
        }
    }
}
