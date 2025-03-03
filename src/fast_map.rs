use std::{cmp::Ordering, ops::Shr};


//const HASH_U64: u64 = 0xB1E19AC35984D6BB;
const HASH_U32: u32 = 0x1758F99D;

/// Specialized HashMap for use in compute_zombie_frequency_data
/// Unfortunately, it's not actually any faster (at least on -O3)
/// but I might as well use it since I wrote it right?
pub struct FastMap {
    table:       Vec<u32>,
    keys:        Vec<u16>,
    entries:     Vec<FastMapEntry>,
    key_len:     u32,
    table_shift: u32,
}

pub struct FastMapEntry {
    hash:       u32,
    f:          f64,
    i:          isize,
    lo_hi_idxs: [u32;2],
}

pub struct FastMapIter<'a> {
    keys:    &'a Vec<u16>,
    entries: &'a mut Vec<FastMapEntry>,
    key_len: u32,
}

impl FastMap {
    pub fn with_size(size: usize, key_len: u32) -> Self {
        let table_size = size.ilog2().clamp(8, 17);
        Self {
            table: vec![0xFFFFFFFF; 1 << table_size],
            keys:  Vec::with_capacity(size.max(256) * key_len as usize * 3),
            entries: Vec::with_capacity(size.max(256) * 3),
            key_len,
            table_shift: 32 - table_size,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn insert_probability(&mut self, key: &[u16], probability: f64, wavepoints: isize) {
        unsafe {
            let mut hash: u32 = 0;
            for short in key {
                hash = hash.wrapping_add(*short as u32).wrapping_mul(HASH_U32);
            }
            let hash_lookup = hash.shr(self.table_shift) as usize;
            let mut ptr     = self.table.get_unchecked_mut(hash_lookup) as *mut u32; //use raw pointer to ignore mutable + mutable borrow
            let mut idx     = *ptr;
            let mut entry: Option<&mut FastMapEntry> = None;
            while idx != 0xFFFFFFFF {
                entry = Some(self.entries.get_unchecked_mut(idx as usize));
                let entry_unwrapped = entry.as_mut().unwrap_unchecked();
                let idx_idx = match entry_unwrapped.hash.cmp(&hash) {
                    Ordering::Equal => {
                        let range_start = idx as usize * self.key_len as usize;
                        if self.keys.get_unchecked(range_start .. range_start + self.key_len as usize) == key {
                            break;
                        } else {
                            0usize
                        }
                    }
                    Ordering::Less => 0usize,
                    Ordering::Greater => 1usize,
                };
                ptr = &mut *entry_unwrapped.lo_hi_idxs.get_unchecked_mut(idx_idx) as *mut u32;
                idx = *ptr;
                entry = None;
            }
            if let Some(entry) = entry {
                entry.f += probability;
            } else {
                let new_idx = self.entries.len();
                self.keys.extend(key.iter());
                *ptr = new_idx as u32;
                self.entries.push(FastMapEntry {
                    hash,
                    f: probability,
                    i: wavepoints,
                    lo_hi_idxs: [0xFFFFFFFF; 2],
                })
            }
        }
    }
}

impl<'a> IntoIterator for &'a mut FastMap {
    type Item = (&'a [u16], (f64, isize));
    type IntoIter = FastMapIter<'a>;
    
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            keys:    &self.keys,
            entries: &mut self.entries,
            key_len: self.key_len,
        }
    }
}

impl<'a> Iterator for FastMapIter<'a> {
    type Item = (&'a [u16], (f64, isize));
    
    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries.pop()?;
        let key_off = self.entries.len() * self.key_len as usize;
        let key = &self.keys[key_off .. key_off + self.key_len as usize];
        Some((key, (entry.f, entry.i)))
    }
}
