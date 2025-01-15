use core::ffi::CStr;
use std::collections::HashMap;
use super::util::OffSiz;
use smallvec::SmallVec;


#[derive(Clone)]
pub struct Pe {
    pub sections:              HashMap<String,PeSection>,
    pub base:                  u64,
    pub code_registration:     usize,
    pub metadata_registration: usize,
}

#[derive(Clone)]
pub struct PeSection {
    pub name:    String,
    pub vrange:  OffSiz,
    pub prange:  OffSiz,
    pub is_data: bool,
    pub is_text: bool,
}
impl PeSection {
    fn from_bytes(bytes: &[u8]) -> Self {
        let section_flags = u32::from_le_bytes(bytes[0x24..0x28].try_into().unwrap());
        PeSection {
            name:   CStr::from_bytes_until_nul(&bytes[0x00..0x08]).unwrap().to_string_lossy().to_string(),
            vrange: OffSiz::from_bytes_rev(&bytes[0x08..0x10]),
            prange: OffSiz::from_bytes_rev(&bytes[0x10..0x18]),
            is_data: section_flags & 0x42000040 == 0x40000040,
            is_text: section_flags & 0x60000020 == 0x60000020,
        }
    }
}

//pub struct MetadataRegistration {
//    generic_classes_count: i64,
//    generic_classes_off:   u64,
//    
//}

impl Pe {
    pub fn new(bytes: &[u8], type_cnt: u64, image_cnt: u32) -> Result<Self, String> {
        if u16::from_le_bytes(bytes[0..2].try_into().unwrap()) != 0x5A4D {
            return Err("DLL has incorrect magic number".into());
        }
        let header_off    = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
        let section_cnt   = u16::from_le_bytes(bytes[header_off+6 .. header_off+8].try_into().unwrap()) as usize;
        let section_slice = &bytes[header_off+0x108..header_off+0x108+0x28*section_cnt];
        
        let mut sections: HashMap<String,PeSection> = HashMap::with_capacity(section_cnt);
        
        for i in (0..section_slice.len()).step_by(0x28) {
            let section = PeSection::from_bytes(&section_slice[i..i+0x28]);
            sections.insert(section.name.clone(), section);
        }
        
        let base = u64::from_le_bytes(bytes[header_off+0x30 .. header_off+0x38].try_into().unwrap());
        
        let (code_registration, ptr_in_exec) = Self::find_code_registration(bytes, &sections, base, image_cnt)?;
        //println!("{:x}", code_registration);
        
        Ok(Self {
            metadata_registration: Self::find_metadata_registration(bytes, &sections, base, type_cnt, ptr_in_exec)?,
            code_registration,
            base,
            sections,
        })
    }
    
    fn find_code_registration(bytes: &[u8], sections: &HashMap<String,PeSection>, dll_base: u64, image_cnt: u32) -> Result<(usize,bool), String> {
        let string_bytes = "mscorlib.dll".as_bytes();
        for section in [sections.get(".data").unwrap(), sections.get(".rdata").unwrap(), sections.get(".text").unwrap()] {
            let ptr_in_exec = section.is_text;
            for string in section.prange.as_slice_of(bytes).windows(string_bytes.len()).enumerate() {
                if string.1 == string_bytes {
                    let dllva = string.0 as u64 + section.vrange.off as u64 + dll_base;
                    for refva1 in Self::find_ref_va(bytes, sections, dll_base, dllva) {
                        for refva2 in Self::find_ref_va(bytes, sections, dll_base, refva1) {
                            for i in (0..image_cnt).rev() {
                                for refva3 in Self::find_ref(bytes, sections, refva2 - i as u64 * 8) {
                                    if refva3 >= 8 && refva3 <= bytes.len() && u64::from_le_bytes(bytes[refva3-8..refva3].try_into().unwrap()) == image_cnt as u64 {
                                        return Ok((refva3-16*8, ptr_in_exec)); //this is supposed to be 14 not 16, but 14 doesn't work. I have questioned this and am very confused
                                    }
                                }
                            }
                        }
                    }
                } 
            }
        }
        Err("Could not find code registration".into())
    }
    
    fn find_metadata_registration(bytes: &[u8], sections: &HashMap<String,PeSection>, dll_base: u64, type_cnt: u64, ptr_in_exec: bool) -> Result<usize, String> {
        for section in sections.values() {
            let range = OffSiz {
                off: section.prange.off,
                siz: section.prange.siz - 24,
            };
            for i in range.as_range().step_by(8) {
                if u64::from_le_bytes(bytes[i..i+8].try_into().unwrap()) == type_cnt && u64::from_le_bytes(bytes[i+16..i+24].try_into().unwrap()) == type_cnt {
                    let ptr          = Self::map_v2p_internal(u64::from_le_bytes(bytes[i+24..i+32].try_into().unwrap()), dll_base, sections).unwrap();
                    let mut tmp_bool1 = false;
                    
                    for section in sections.values() {
                        if section.is_data && section.prange.as_range().contains(&ptr) {
                            tmp_bool1 = true;
                        }
                    }
                    
                    if tmp_bool1 {
                        for j in 0..type_cnt as usize {
                            let ptr2 = if ptr+j*8 < bytes.len() {
                                Self::map_v2p_internal(u64::from_le_bytes(bytes[ptr+j*8..ptr+j*8+8].try_into().unwrap()), dll_base, sections).unwrap_or(0)
                            } else {
                                0
                            };
                            let mut tmp_bool2 = false;
                            
                            if ptr_in_exec {
                                for section in sections.values() {
                                    if section.is_text && section.prange.as_range().contains(&ptr2) {
                                        tmp_bool2 = true;
                                    }
                                }
                            } else {
                                for section in sections.values() {
                                    if section.is_data && section.prange.as_range().contains(&ptr2) {
                                        tmp_bool2 = true;
                                    }
                                }
                            }
                            tmp_bool1 = tmp_bool1 && tmp_bool2
                        }
                        
                        if tmp_bool1 {
                            return Ok(i-80);
                        }
                    }
                }
            }
        }
        Err("Could not find metadata registration".into())
    }
    
    fn find_ref(bytes: &[u8], sections: &HashMap<String,PeSection>, addr: u64) -> SmallVec<[usize;4]> {
        let mut ret = SmallVec::new();
        for section in sections.values() {
            if section.name.contains("data") {
                for i in section.prange.as_range().step_by(8) {
                    if u64::from_le_bytes(bytes[i..i+8].try_into().unwrap()) == addr {
                        ret.push(i);
                    }
                }
            }
        }
        ret
    }
    
    fn find_ref_va(bytes: &[u8], sections: &HashMap<String,PeSection>, dll_base: u64, addr: u64) -> SmallVec<[u64;4]> {
        let mut ret = SmallVec::new();
        for section in sections.values() {
            if section.is_data {
                for i in section.prange.as_range().step_by(8) {
                    if u64::from_le_bytes(bytes[i..i+8].try_into().unwrap()) == addr {
                        ret.push(i as u64 - section.prange.off as u64 + section.vrange.off as u64 + dll_base);
                    }
                }
            }
        }
        ret
    }
    
    fn map_v2p_internal(addr: u64, dll_base: u64, sections: &HashMap<String,PeSection>) -> Option<usize> {
        if addr >= dll_base {
            for section in sections.values() {
                if section.vrange.as_range().contains(&((addr-dll_base) as usize)) {
                    return Some((addr - section.vrange.off as u64 + section.prange.off as u64 - dll_base) as usize);
                }
            }
        }
        None
    }
    pub fn map_v2p(&self, addr: u64) -> Option<usize> {
        if addr >= self.base {
            for section in self.sections.values() {
                if section.vrange.as_range().contains(&((addr-self.base) as usize)) {
                    return Some((addr - section.vrange.off as u64 + section.prange.off as u64 - self.base) as usize);
                }
            }
        }
        None
    }
    pub fn map_v2p_data(&self, addr: u64) -> Option<usize> {
        if addr >= self.base {
            for section in self.sections.values() {
                if section.is_data && section.vrange.as_range().contains(&((addr-self.base) as usize)) {
                    return Some((addr - section.vrange.off as u64 + section.prange.off as u64 - self.base) as usize);
                }
            }
        }
        None
    }
    pub fn map_p2v(&self, addr: usize) -> Option<u64> {
        for section in self.sections.values() {
            if section.prange.as_range().contains(&addr) {
                return Some(addr as u64 + section.vrange.off as u64 - section.prange.off as u64 + self.base);
            }
        }
        None
    }
}
