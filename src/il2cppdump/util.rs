#[macro_export]
macro_rules! format_to {
    ($buf:expr) => ();
    ($buf:expr, $lit:literal $($arg:tt)*) => {
        {
            use ::std::fmt::Write as _;
            _ = $buf.write_fmt(format_args!($lit $($arg)*))
        }
    };
}

pub struct OffSiz {
    pub off: u32,
    pub siz: u32,
}
impl OffSiz {
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            off: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            siz: u32::from_le_bytes(data[4..8].try_into().unwrap()),
        }
    }
    pub fn from_bytes_rev(data: &[u8]) -> Self {
        Self {
            siz: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            off: u32::from_le_bytes(data[4..8].try_into().unwrap()),
        }
    }
    pub fn as_range(&self) -> core::ops::Range<usize> {
        self.off as usize..(self.off+self.siz) as usize
    }
    pub fn as_slice_of<'a>(&self, in_slice: &'a[u8]) -> &'a [u8] {
        &in_slice[self.as_range()]
    }
}
