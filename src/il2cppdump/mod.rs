pub mod pe;
pub mod util;
pub mod disasm;

use core::ffi::CStr;
use core::mem::size_of;
use core::mem::transmute;
use std::cmp::max;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::available_parallelism;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::num::NonZero;
use std::path::PathBuf;
use std::collections::HashMap;
use std::fs::read;
use pe::Pe;
use util::OffSiz;

use crate::format_to;

#[allow(dead_code)]
#[derive(Clone)]
pub struct IL2CppDumper {
    string_literals:           OffSiz,
    string_literals_data:      OffSiz,
    strings:                   OffSiz,
    events:                    OffSiz,
    properties:                OffSiz,
    methods:                   OffSiz,
    parameter_defaults:        OffSiz,
    field_defaults:            OffSiz,
    field_parameter_defaults:  OffSiz,
    field_marshaled_sizes:     OffSiz,
    parameters:                OffSiz,
    fields:                    OffSiz,
    generic_parameters:        OffSiz,
    generic_constraints:       OffSiz,
    generic_containers:        OffSiz,
    nested_types:              OffSiz,
    interfaces:                OffSiz,
    vtable_methods:            OffSiz,
    interface_offsets:         OffSiz,
    type_definitions:          OffSiz,
    images:                    OffSiz,
    assemblies:                OffSiz,
    field_refs:                OffSiz,
    referenced_assemblies:     OffSiz,
    attribute_data:            OffSiz,
    attribute_data_range:      OffSiz,
    uvcp_types:                OffSiz,
    uvcp_ranges:               OffSiz,
    win_runtime_type_names:    OffSiz,
    win_runtime_strings:       OffSiz,
    exported_type_definitions: OffSiz,
    
    //strings_array:   Vec<*const CStr>,
    pub methods_array:               Vec<Method>,
    pub methods_table:       HashMap<String,u32>,
    pub method_addr_table:      HashMap<u64,u32>,
    field_default_lookup:       HashMap<i32,u32>,
    icgm_array:                        Vec<Icgm>,
    types_array:                 Vec<IL2CppType>,
    generic_class_array: Vec<IL2CppGenericClass>,
    
    version:                       u32,
    metadata:                  Vec<u8>,
    assembly:                  Vec<u8>,
    
    code_reg:         CodeRegistration,
    meta_reg:     MetadataRegistration,
    
    pub pe:                         Pe,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct MetadataRegistration {
    generic_classes_cnt:       u32,
    generic_classes:         usize,
    generic_insts_cnt:         u32,
    generic_insts:           usize,
    generic_method_table_cnt:  u32,
    generic_method_table:    usize,
    types_cnt:                 u32,
    types:                   usize,
    method_specs_cnt:          u32,
    method_specs:            usize,
    field_offsets_cnt:         u32,
    field_offsets:           usize,
    type_definition_sizes_cnt: u32,
    type_definition_sizes:   usize,
    metadata_usages_cnt:       u32,
    metadata_usages:         usize,
}
impl MetadataRegistration {
    fn new(data: &[u8], pe: &Pe) -> Self {
        let o = pe.metadata_registration;
        Self {
            generic_classes_cnt:                  u64::from_le_bytes(data[o     ..o+0x08].try_into().unwrap()) as u32,
            generic_classes:           pe.map_v2p(u64::from_le_bytes(data[o+0x08..o+0x10].try_into().unwrap())).unwrap_or(0),
            generic_insts_cnt:                    u64::from_le_bytes(data[o+0x10..o+0x18].try_into().unwrap()) as u32,
            generic_insts:             pe.map_v2p(u64::from_le_bytes(data[o+0x18..o+0x20].try_into().unwrap())).unwrap_or(0),
            generic_method_table_cnt:             u64::from_le_bytes(data[o+0x20..o+0x28].try_into().unwrap()) as u32,
            generic_method_table:      pe.map_v2p(u64::from_le_bytes(data[o+0x28..o+0x30].try_into().unwrap())).unwrap_or(0),
            types_cnt:                            u64::from_le_bytes(data[o+0x30..o+0x38].try_into().unwrap()) as u32,
            types:                     pe.map_v2p(u64::from_le_bytes(data[o+0x38..o+0x40].try_into().unwrap())).unwrap_or(0),
            method_specs_cnt:                     u64::from_le_bytes(data[o+0x40..o+0x48].try_into().unwrap()) as u32,
            method_specs:              pe.map_v2p(u64::from_le_bytes(data[o+0x48..o+0x50].try_into().unwrap())).unwrap_or(0),
            field_offsets_cnt:                    u64::from_le_bytes(data[o+0x50..o+0x58].try_into().unwrap()) as u32,
            field_offsets:             pe.map_v2p(u64::from_le_bytes(data[o+0x58..o+0x60].try_into().unwrap())).unwrap_or(0),
            type_definition_sizes_cnt:            u64::from_le_bytes(data[o+0x60..o+0x68].try_into().unwrap()) as u32,
            type_definition_sizes:     pe.map_v2p(u64::from_le_bytes(data[o+0x68..o+0x70].try_into().unwrap())).unwrap_or(0),
            metadata_usages_cnt:                  u64::from_le_bytes(data[o+0x70..o+0x78].try_into().unwrap()) as u32,
            metadata_usages:           pe.map_v2p(u64::from_le_bytes(data[o+0x78..o+0x80].try_into().unwrap())).unwrap_or(0),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct IL2CppType {
    data:            u64,
    attrs:           u16,
    kind: IL2CppTypeEnum,
    num_mods:         u8,
    byref:          bool,
    pinned:         bool,
    valuetype:      bool,
}
#[allow(dead_code)]
#[repr(u8)]
#[derive(Clone)]
#[non_exhaustive]
enum IL2CppTypeEnum {
    End         = 0x00,
    Void        = 0x01,
    Bool        = 0x02,
    Char        = 0x03,
    I8          = 0x04,
    U8          = 0x05,
    I16         = 0x06,
    U16         = 0x07,
    I32         = 0x08,
    U32         = 0x09,
    I64         = 0x0a,
    U64         = 0x0b,
    F32         = 0x0c,
    F64         = 0x0d,
    String      = 0x0e,
    Ptr         = 0x0f, // Arg: <Type> Token
    ByRef       = 0x10, // Arg: <Type> Token
    ValueType   = 0x11, // Arg: <Type> Token
    Class       = 0x12, // Arg: <Type> Token
    Var         = 0x13, // Generic parameter in a generic type definition, represented as number (compressed unsigned integer) number
    Array       = 0x14, // Type, Rank, Boundscount, Bound1, Locount, Lo1
    GenericInst = 0x15, // <Type> <Type-Arg-Count> <Type-1> \X{2026} <Type-N>
    TypedByRef  = 0x16,
    I           = 0x18,
    U           = 0x19,
    FnPtr       = 0x1b, // Arg: Full Method Signature
    Object      = 0x1c,
    SzArray     = 0x1d, // 0-Based One-Dim-Array
    MVar        = 0x1e, // Generic Parameter In A Generic Method Definition, Represented As Number (Compressed Unsigned Integer)
    CmodReqd    = 0x1f, // Arg: Typedef Or Typeref Token
    CmodOpt     = 0x20, // Optional Arg: Typedef Or Typref Token
    Internal    = 0x21, // Clr Internal Type
    
    Modifier    = 0x40, // Or With The Following Types
    Sentinel    = 0x41, // Sentinel For Varargs Method Signature
    Pinned      = 0x45, // Local Var That Points To Pinned Object
    
    Enum        = 0x55, // An Enumeration
    TypeIndex   = 0xff, // An Index Into IL2Cpp Type Metadata Table
}
const TYPE_STRIDE: usize = 16;
impl IL2CppType {
    fn from_bytes(bytes: &[u8]) -> Self {
        let bits = u32::from_le_bytes(bytes[0x8..0xc].try_into().unwrap());
        Self {
            data:      u64::from_le_bytes(bytes[0x0..0x8].try_into().unwrap()),
            attrs:     bits as u16,
            kind:      unsafe {transmute::<u8,IL2CppTypeEnum>((bits >> 16) as u8)},
            num_mods:  ((bits >> 24) & 0x1f) as u8,
            byref:     (bits >> 29) & 0x1 == 1,
            pinned:    (bits >> 30) & 0x1 == 1,
            valuetype: (bits >> 31) & 0x1 == 1,
        }
    }
    fn name(&self, meta: &IL2CppDumper) -> (String, Option<u32>) {
        self.name_helper(meta, false)
    }
    fn name_helper(&self, meta: &IL2CppDumper, nested: bool) -> (String, Option<u32>) {
        let mut generic: Option<u32> = None;
        #[allow(unreachable_patterns)]
        let mut ret = match self.kind {
            IL2CppTypeEnum::End         => "?00".to_owned(),
            IL2CppTypeEnum::Void        => "c_void".to_owned(),
            IL2CppTypeEnum::Bool        => "bool".to_owned(),
            IL2CppTypeEnum::Char        => "char".to_owned(),
            IL2CppTypeEnum::I8          => "i8".to_owned(),
            IL2CppTypeEnum::U8          => "u8".to_owned(),
            IL2CppTypeEnum::I16         => "i16".to_owned(),
            IL2CppTypeEnum::U16         => "u16".to_owned(),
            IL2CppTypeEnum::I32         => "i32".to_owned(),
            IL2CppTypeEnum::U32         => "u32".to_owned(),
            IL2CppTypeEnum::I64         => "i64".to_owned(),
            IL2CppTypeEnum::U64         => "u64".to_owned(),
            IL2CppTypeEnum::F32         => "f32".to_owned(),
            IL2CppTypeEnum::F64         => "f64".to_owned(),
            IL2CppTypeEnum::String      => "String".to_owned(),
            IL2CppTypeEnum::Ptr         => {
                let type_off = meta.pe.map_v2p(self.data).unwrap();
                let name     = IL2CppType::from_bytes(&meta.assembly[type_off..type_off+12]).name(meta);
                generic      = name.1;
                format!("*mut {}", name.0)
            },
            IL2CppTypeEnum::ByRef       => "?02".to_owned(),
            IL2CppTypeEnum::ValueType   => self.get_type_name(meta, nested),
            IL2CppTypeEnum::Class       => self.get_type_name(meta, nested),
            IL2CppTypeEnum::Var         => {
                generic           = Some(self.data as u32);
                let slice         = meta.generic_parameters.as_slice_of(&meta.metadata);
                let off           = self.data as usize * GENERIC_PARAMETER_STRIDE;
                let generic_param = IL2CppGenericParameter::from_bytes(&slice[off..off+GENERIC_PARAMETER_STRIDE]);
                meta.get_string(generic_param.name_off).to_string()
            },
            IL2CppTypeEnum::Array       => {
                let array_info_off = meta.pe.map_v2p(self.data).unwrap();
                let type_off       = meta.pe.map_v2p(u64::from_le_bytes(meta.assembly[array_info_off..array_info_off+8].try_into().unwrap())).unwrap();
                let name           = IL2CppType::from_bytes(&meta.assembly[type_off..type_off+12]).name(meta);
                generic            = name.1;
                format!("[{}; {}]", name.0, meta.assembly[array_info_off+8])
            },
            IL2CppTypeEnum::GenericInst => self.get_type_name(meta, nested),//"?07".to_owned(),//{format!("?07 {}", self.get_type_name(&meta))},
            IL2CppTypeEnum::TypedByRef  => "typed reference".to_owned(),
            IL2CppTypeEnum::I           => "isize".to_owned(),
            IL2CppTypeEnum::U           => "usize".to_owned(),
            IL2CppTypeEnum::FnPtr       => "fn ?11".to_owned(),
            IL2CppTypeEnum::Object      => "object".to_owned(),
            IL2CppTypeEnum::SzArray     => {
                let type_off = meta.pe.map_v2p(self.data).unwrap();
                let name     = IL2CppType::from_bytes(&meta.assembly[type_off..type_off+12]).name(meta);
                generic      = name.1;
                format!("[{}]", name.0)
            },
            IL2CppTypeEnum::MVar        => {
                generic           = Some(self.data as u32);
                let slice         = meta.generic_parameters.as_slice_of(&meta.metadata);
                let off           = self.data as usize * GENERIC_PARAMETER_STRIDE;
                let generic_param = IL2CppGenericParameter::from_bytes(&slice[off..off+GENERIC_PARAMETER_STRIDE]);
                meta.get_string(generic_param.name_off).to_string()
            },
            IL2CppTypeEnum::CmodReqd    => "?15".to_owned(),
            IL2CppTypeEnum::CmodOpt     => "?16".to_owned(),
            IL2CppTypeEnum::Internal    => "?17".to_owned(),
            IL2CppTypeEnum::Modifier    => "?18".to_owned(),
            IL2CppTypeEnum::Sentinel    => "?19".to_owned(),
            IL2CppTypeEnum::Pinned      => "?20".to_owned(),
            IL2CppTypeEnum::Enum        => "?21".to_owned(),
            IL2CppTypeEnum::TypeIndex   => "?22".to_owned(),
            _                           => "!!!".to_owned(),
        };
        
        if self.byref {ret = format!("&{ret}");}
        
        (ret, generic)
    }
    
    fn get_type_name(&self, meta: &IL2CppDumper, nested: bool) -> String {
        let il2cppstruct = match self.kind {
            IL2CppTypeEnum::GenericInst => {
                let idx = meta.pe.map_v2p(self.data).unwrap();
                let generic_class = IL2CppGenericClass::from_bytes(&meta.assembly[idx..idx+GENERIC_CLASS_STRIDE], &meta.pe);
                let idx = generic_class.type_off;
                let typ = IL2CppType::from_bytes(&meta.assembly[idx..idx+TYPE_STRIDE]);
                let idx = typ.data as usize;
                IL2CppStruct::from_bytes(&meta.type_definitions.as_slice_of(&meta.metadata)[idx*STRUCT_STRIDE..idx*STRUCT_STRIDE+STRUCT_STRIDE])
            },
            _ => {
                let idx = self.data as usize;
                IL2CppStruct::from_bytes(&meta.type_definitions.as_slice_of(&meta.metadata)[idx*STRUCT_STRIDE..idx*STRUCT_STRIDE+STRUCT_STRIDE])
            }
        };
        let mut struct_name = il2cppstruct.get_name(meta);
        
        if nested {
            return struct_name;
        }
        match self.kind {
            IL2CppTypeEnum::GenericInst => {
                let idx = meta.pe.map_v2p(self.data).unwrap();
                let generic_class = IL2CppGenericClass::from_bytes(&meta.assembly[idx..idx+GENERIC_CLASS_STRIDE], &meta.pe);
                let idx = meta.pe.map_v2p(generic_class.class_inst).unwrap();
                let c   = u64::from_le_bytes(meta.assembly[idx..idx+0x08].try_into().unwrap()) as usize;
                let v   = meta.pe.map_v2p(u64::from_le_bytes(meta.assembly[idx+0x08..idx+0x10].try_into().unwrap())).unwrap();
                let mut generics = "".to_owned();
                for i in 0..c {
                    let idx = meta.pe.map_v2p(u64::from_le_bytes(meta.assembly[v+i*8..v+i*8+8].try_into().unwrap())).unwrap();
                    let typ = IL2CppType::from_bytes(&meta.assembly[idx..idx+TYPE_STRIDE]);
                    if generics.is_empty() {
                        generics = typ.name(meta).0;
                    } else {
                        generics = format!("{generics}, {}", typ.name(meta).0);
                    }
                }
                struct_name = format!("{struct_name}<{generics}>")
            },
            _ => {
                if il2cppstruct.generic_container_idx >= 0 {
                    struct_name += "?23";
                }
            }
        }
        
        struct_name
    }
    
    fn get_struct(&self) -> Option<u32> {
        match self.kind {
            IL2CppTypeEnum::ValueType |
            IL2CppTypeEnum::Class => {
                Some(self.data as u32)
            }
            _ => {None}
        }
    }
    fn get_struct_noref(&self) -> Option<u32> {
        if self.byref {
            None
        } else {
            self.get_struct()
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct IL2CppGenericClass {
    type_off:  usize,
    class_inst:  u64,
    method_inst: u64,
    class:     usize,
}
const GENERIC_CLASS_STRIDE: usize = 0x20;
impl IL2CppGenericClass {
    fn from_bytes(bytes: &[u8], pe: &Pe) -> Self {
        Self {
            type_off: pe.map_v2p(u64::from_le_bytes(bytes[0x00..0x08].try_into().unwrap())).unwrap_or(0),
            class_inst:          u64::from_le_bytes(bytes[0x08..0x10].try_into().unwrap()),
            method_inst:         u64::from_le_bytes(bytes[0x10..0x18].try_into().unwrap()),
            class:    pe.map_v2p(u64::from_le_bytes(bytes[0x18..0x20].try_into().unwrap())).unwrap_or(0),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct CodeRegistration {
    reverse_p_invoke_wrapper_cnt:    u32,
    generic_method_ptrs_cnt:         u32,
    invoker_ptrs_cnt:                u32,
    unresolved_vcall_cnt:            u32,
    interop_data_cnt:                u32,
    windows_runtime_factory_cnt:     u32,
    code_gen_modules_cnt:            u32,
    reverse_p_invoke_wrappers:     usize,
    generic_method_ptrs:           usize,
    generic_adjustor_thunks:       usize,
    invoker_ptrs:                  usize,
    unresolved_vcall_ptrs:         usize,
    unresolved_icall_ptrs:         usize,
    unresolved_scall_ptrs:         usize,
    interop_data:                  usize,
    windows_runtime_factory_table: usize,
    code_gen_modules:              usize,
}
impl CodeRegistration {
    fn new(data: &[u8], pe: &Pe) -> Self {
        let o = pe.code_registration;
        Self {
            reverse_p_invoke_wrapper_cnt:             u64::from_le_bytes(data[o     ..o+0x08].try_into().unwrap()) as u32,
            reverse_p_invoke_wrappers:     pe.map_v2p(u64::from_le_bytes(data[o+0x08..o+0x10].try_into().unwrap())).unwrap_or(0),
            generic_method_ptrs_cnt:                  u64::from_le_bytes(data[o+0x10..o+0x18].try_into().unwrap()) as u32,
            generic_method_ptrs:           pe.map_v2p(u64::from_le_bytes(data[o+0x18..o+0x20].try_into().unwrap())).unwrap_or(0),
            generic_adjustor_thunks:       pe.map_v2p(u64::from_le_bytes(data[o+0x20..o+0x28].try_into().unwrap())).unwrap_or(0),
            invoker_ptrs_cnt:                         u64::from_le_bytes(data[o+0x28..o+0x30].try_into().unwrap()) as u32,
            invoker_ptrs:                  pe.map_v2p(u64::from_le_bytes(data[o+0x30..o+0x38].try_into().unwrap())).unwrap_or(0),
            unresolved_vcall_cnt:                     u64::from_le_bytes(data[o+0x38..o+0x40].try_into().unwrap()) as u32,
            unresolved_vcall_ptrs:         pe.map_v2p(u64::from_le_bytes(data[o+0x40..o+0x48].try_into().unwrap())).unwrap_or(0),
            unresolved_icall_ptrs:         pe.map_v2p(u64::from_le_bytes(data[o+0x48..o+0x50].try_into().unwrap())).unwrap_or(0),
            unresolved_scall_ptrs:         pe.map_v2p(u64::from_le_bytes(data[o+0x50..o+0x58].try_into().unwrap())).unwrap_or(0),
            interop_data_cnt:                         u64::from_le_bytes(data[o+0x58..o+0x60].try_into().unwrap()) as u32,
            interop_data:                  pe.map_v2p(u64::from_le_bytes(data[o+0x60..o+0x68].try_into().unwrap())).unwrap_or(0),
            windows_runtime_factory_cnt:              u64::from_le_bytes(data[o+0x68..o+0x70].try_into().unwrap()) as u32,
            windows_runtime_factory_table: pe.map_v2p(u64::from_le_bytes(data[o+0x70..o+0x78].try_into().unwrap())).unwrap_or(0),
            code_gen_modules_cnt:                     u64::from_le_bytes(data[o+0x78..o+0x80].try_into().unwrap()) as u32,
            code_gen_modules:              pe.map_v2p(u64::from_le_bytes(data[o+0x80..o+0x88].try_into().unwrap())).unwrap_or(0),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct CodeGenModule {
    method_ptrs_cnt:                    u32,
    adjustor_thunks_cnt:                u32,
    reverse_p_invoke_wrapper_cnt:       u32,
    rgctx_ranges_cnt:                   u32,
    rgctxs_cnt:                         u32,
    name_off:                         usize,
    method_ptrs_off:                  usize,
    adjustor_thunks_off:              usize,
    invoker_indices:                  usize,
    reverse_p_invoke_wrapper_indices: usize,
    rgctx_ranges:                     usize,
    rgctxs:                           usize,
    debugger_metadata:                usize,
    module_initializer:               usize,
    static_constructor_type_indices:  usize,
}
impl CodeGenModule {
    pub fn from_bytes(bytes: &[u8], pe: &Pe) -> Self {
        Self {
            name_off:                         pe.map_v2p(u64::from_le_bytes(bytes[0x00..0x08].try_into().unwrap())).unwrap_or(0),
            method_ptrs_cnt:                             u64::from_le_bytes(bytes[0x08..0x10].try_into().unwrap()) as u32,
            method_ptrs_off:                  pe.map_v2p(u64::from_le_bytes(bytes[0x10..0x18].try_into().unwrap())).unwrap_or(0),
            adjustor_thunks_cnt:                         u64::from_le_bytes(bytes[0x18..0x20].try_into().unwrap()) as u32,
            adjustor_thunks_off:              pe.map_v2p(u64::from_le_bytes(bytes[0x20..0x28].try_into().unwrap())).unwrap_or(0),
            invoker_indices:                  pe.map_v2p(u64::from_le_bytes(bytes[0x28..0x30].try_into().unwrap())).unwrap_or(0),
            reverse_p_invoke_wrapper_cnt:                u64::from_le_bytes(bytes[0x30..0x38].try_into().unwrap()) as u32,
            reverse_p_invoke_wrapper_indices: pe.map_v2p(u64::from_le_bytes(bytes[0x38..0x40].try_into().unwrap())).unwrap_or(0),
            rgctx_ranges_cnt:                            u64::from_le_bytes(bytes[0x40..0x48].try_into().unwrap()) as u32,
            rgctx_ranges:                     pe.map_v2p(u64::from_le_bytes(bytes[0x48..0x50].try_into().unwrap())).unwrap_or(0),
            rgctxs_cnt:                                  u64::from_le_bytes(bytes[0x50..0x58].try_into().unwrap()) as u32,
            rgctxs:                           pe.map_v2p(u64::from_le_bytes(bytes[0x58..0x60].try_into().unwrap())).unwrap_or(0),
            debugger_metadata:                pe.map_v2p(u64::from_le_bytes(bytes[0x60..0x68].try_into().unwrap())).unwrap_or(0),
            module_initializer:               pe.map_v2p(u64::from_le_bytes(bytes[0x68..0x70].try_into().unwrap())).unwrap_or(0),
            static_constructor_type_indices:  pe.map_v2p(u64::from_le_bytes(bytes[0x70..0x78].try_into().unwrap())).unwrap_or(0),
        }
    }
}

#[derive(Clone)]
pub struct Method {
    metadata: IL2CppMethod,
    pub addr: u64,
    pub len:  u64,
    pub typ:  Option<u32>,
}
impl Method {
    pub fn name(&self, meta: &IL2CppDumper) -> String {
        let name = meta.get_string(self.metadata.name_off);
        let mut generic_vec: SmallVec<[u32;2]> = SmallVec::new();
        
        let typ  = {
            if let Some(idx) = self.typ {
                let typ = IL2CppStruct::from_bytes(&meta.type_definitions.as_slice_of(&meta.metadata)[(idx as usize * STRUCT_STRIDE)..(idx as usize * STRUCT_STRIDE)+STRUCT_STRIDE]);
                typ.get_name(meta)+"::"
            } else {
                "".to_owned()
            }
        };
        
        let args = {
            let mut ret = if self.is_static() {
                ""
            } else {
                "&mut self"
            }.to_owned();
            
            if self.metadata.parameter_start >= 0 {
                for arg_idx in self.metadata.parameter_start as usize .. self.metadata.parameter_start as usize + self.metadata.parameter_count as usize {
                    let arg = IL2CppParameter::from_bytes(&meta.parameters.as_slice_of(&meta.metadata)[arg_idx*PARAMETER_STRIDE..arg_idx*PARAMETER_STRIDE+PARAMETER_STRIDE]);
                    let name = meta.get_arg_name(arg.type_idx);
                    if let Some(generic_idx) = name.1 {
                        if !generic_vec.contains(&generic_idx) {
                            generic_vec.push(generic_idx);
                        }
                    }
                    if ret.is_empty() {
                        ret = format!("{}: {}", meta.get_string(arg.name_off), name.0);
                    } else {
                        ret = format!("{ret}, {}: {}", meta.get_string(arg.name_off), name.0);
                    }
                }
            }
            
            ret
        };
        
        let return_type = {
            let return_type_name = meta.get_arg_name(self.metadata.return_type);
            if return_type_name.0 == "c_void" {
                "".to_owned()
            } else {
                if let Some(generic_idx) = return_type_name.1 {
                    if !generic_vec.contains(&generic_idx) {
                        generic_vec.push(generic_idx);
                    }
                }
                format!(" -> {}", return_type_name.0)
            }
        };
        
        if generic_vec.is_empty() {
            format!("{typ}{name}({args}){return_type}")
        } else {
            let mut generics = "".to_owned();
            let slice    = meta.generic_parameters.as_slice_of(&meta.metadata);
            for idx in &generic_vec {
                let off           = *idx as usize * GENERIC_PARAMETER_STRIDE;
                let generic_param = IL2CppGenericParameter::from_bytes(&slice[off..off+GENERIC_PARAMETER_STRIDE]);
                if !generics.is_empty() {
                    generics += ", "
                }
                generics += &meta.get_string(generic_param.name_off);
            }
            format!("{typ}{name}<{generics}>({args}){return_type}")
        }
    }
    pub fn name_short(&self, meta: &IL2CppDumper) -> String {
        let name = meta.get_string(self.metadata.name_off);
        
        let typ  = {
            if let Some(idx) = self.typ {
                let typ = IL2CppStruct::from_bytes(&meta.type_definitions.as_slice_of(&meta.metadata)[(idx as usize * STRUCT_STRIDE)..(idx as usize * STRUCT_STRIDE)+STRUCT_STRIDE]);
                typ.get_name(meta)+"::"
            } else {
                "".to_owned()
            }
        };
        
        format!("{typ}{name}")
    }
    fn is_static(&self) -> bool {
        self.metadata.flags & 0x10 != 0
    }
}

#[repr(C)]
#[derive(Clone)]
struct IL2CppMethod {
    name_off:              u32,
    declaring_type:        i32,
    return_type:           i32,
    parameter_start:       i32,
    generic_container_idx: i32,
    token:                 u32,
    flags:                 u16,
    iflags:                u16,
    slot:                  u16,
    parameter_count:       u16,
}
const METHOD_STRIDE: usize = size_of::<IL2CppMethod>();
impl IL2CppMethod {
    fn from_bytes(data: &[u8]) -> Self {
        Self {
            name_off:              u32::from_le_bytes(data[0x00..0x04].try_into().unwrap()),
            declaring_type:        i32::from_le_bytes(data[0x04..0x08].try_into().unwrap()),
            return_type:           i32::from_le_bytes(data[0x08..0x0c].try_into().unwrap()),
            parameter_start:       i32::from_le_bytes(data[0x0c..0x10].try_into().unwrap()),
            generic_container_idx: i32::from_le_bytes(data[0x10..0x14].try_into().unwrap()),
            token:                 u32::from_le_bytes(data[0x14..0x18].try_into().unwrap()),
            flags:                 u16::from_le_bytes(data[0x18..0x1a].try_into().unwrap()),
            iflags:                u16::from_le_bytes(data[0x1a..0x1c].try_into().unwrap()),
            slot:                  u16::from_le_bytes(data[0x1c..0x1e].try_into().unwrap()),
            parameter_count:       u16::from_le_bytes(data[0x1e..0x20].try_into().unwrap()),
        }
    }
}

#[repr(C)]
#[derive(Clone)]
struct IL2CppStruct {
    name_off:                u32,
    namespace_off:           u32,
    byval_type_idx:          i32,
    declaring_type_idx:      i32,
    parent_idx:              i32,
    element_type_idx:        i32,
    generic_container_idx:   i32,
    flags:                   u32,
    field_start:             i32,
    method_start:            i32,
    event_start:             i32,
    property_start:          i32,
    nested_types_start:      i32,
    interfaces_start:        i32,
    vtable_start:            i32,
    interface_offsets_start: i32,
    method_count:            u16,
    property_count:          u16,
    field_count:             u16,
    event_count:             u16,
    nested_type_count:       u16,
    vtable_count:            u16,
    interfaces_count:        u16,
    interface_offsets_count: u16,
    bitfield:                u32,
    token:                   u32,
}
const STRUCT_STRIDE: usize = size_of::<IL2CppStruct>();
impl IL2CppStruct {
    fn from_bytes(data: &[u8]) -> Self {
        Self {
            name_off:                u32::from_le_bytes(data[0x00..0x04].try_into().unwrap()),
            namespace_off:           u32::from_le_bytes(data[0x04..0x08].try_into().unwrap()),
            byval_type_idx:          i32::from_le_bytes(data[0x08..0x0c].try_into().unwrap()),
            declaring_type_idx:      i32::from_le_bytes(data[0x0c..0x10].try_into().unwrap()),
            parent_idx:              i32::from_le_bytes(data[0x10..0x14].try_into().unwrap()),
            element_type_idx:        i32::from_le_bytes(data[0x14..0x18].try_into().unwrap()),
            generic_container_idx:   i32::from_le_bytes(data[0x18..0x1c].try_into().unwrap()),
            flags:                   u32::from_le_bytes(data[0x1c..0x20].try_into().unwrap()),
            field_start:             i32::from_le_bytes(data[0x20..0x24].try_into().unwrap()),
            method_start:            i32::from_le_bytes(data[0x24..0x28].try_into().unwrap()),
            event_start:             i32::from_le_bytes(data[0x28..0x2c].try_into().unwrap()),
            property_start:          i32::from_le_bytes(data[0x2c..0x30].try_into().unwrap()),
            nested_types_start:      i32::from_le_bytes(data[0x30..0x34].try_into().unwrap()),
            interfaces_start:        i32::from_le_bytes(data[0x34..0x38].try_into().unwrap()),
            vtable_start:            i32::from_le_bytes(data[0x38..0x3c].try_into().unwrap()),
            interface_offsets_start: i32::from_le_bytes(data[0x3c..0x40].try_into().unwrap()),
            method_count:            u16::from_le_bytes(data[0x40..0x42].try_into().unwrap()),
            property_count:          u16::from_le_bytes(data[0x42..0x44].try_into().unwrap()),
            field_count:             u16::from_le_bytes(data[0x44..0x46].try_into().unwrap()),
            event_count:             u16::from_le_bytes(data[0x46..0x48].try_into().unwrap()),
            nested_type_count:       u16::from_le_bytes(data[0x48..0x4a].try_into().unwrap()),
            vtable_count:            u16::from_le_bytes(data[0x4a..0x4c].try_into().unwrap()),
            interfaces_count:        u16::from_le_bytes(data[0x4c..0x4e].try_into().unwrap()),
            interface_offsets_count: u16::from_le_bytes(data[0x4e..0x50].try_into().unwrap()),
            bitfield:                u32::from_le_bytes(data[0x50..0x54].try_into().unwrap()),
            token:                   u32::from_le_bytes(data[0x54..0x58].try_into().unwrap()),
        }
    }
    fn get_name(&self, meta: &IL2CppDumper) -> String {
        let mut ret = if self.declaring_type_idx >= 0 {
            meta.types_array[self.declaring_type_idx as usize].name_helper(meta, true).0 + "::"
        } else {
            let namespace = meta.get_string(self.namespace_off).to_string();
            if namespace.is_empty() {
                namespace
            } else {
                namespace + "::"
            }
        };
        if self.name_off < meta.strings.siz {
            let mut tmp = meta.get_string(self.name_off).to_string();
            if let Some(idx) = tmp.find("`") {
                tmp.truncate(idx);
            }
            ret += &tmp
        } else {
            ret += "unknown"
        };
        ret
    }
    
    fn get_field_name_at_off(&self, idx: u32, off: u64, meta: &IL2CppDumper) -> String {
        self.get_field_name_at_off_internal(idx, off, 0, meta)
    }
    
    fn get_field_name_at_off_internal(&self, idx: u32, off: u64, recursion: u32, meta: &IL2CppDumper) -> String {
        if let Some((field, field_off, _strukt_idx)) = self.get_field_type_at_off(idx, off, meta) {
            let mut ret = meta.get_string(field.name_off).to_string();
            
            if let Some(struct_idx) = meta.types_array[field.type_idx as usize].get_struct_noref() {
                let strukt = IL2CppStruct::from_bytes(
                    &meta.type_definitions.as_slice_of(&meta.metadata)
                    [struct_idx as usize * STRUCT_STRIDE .. struct_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                );
                if strukt.flags & 0x2000 != 0 && recursion < 5 {
                //if recursion < 5 {
                    format_to!(ret, ".{}", strukt.get_field_name_at_off_internal(struct_idx, off - field_off as u64, recursion + 1, meta))
                } else if field_off != off as u32 {
                    format_to!(ret, "+0x{:X}", off as u32 - field_off)
                }
            } else if field_off != off as u32 {
                format_to!(ret, "+0x{:X}", off as u32 - field_off)
            }
            
            ret
        } else {
            "".to_owned()
        }
    }
    
    fn get_field_type_at_off(&self, idx: u32, off: u64, meta: &IL2CppDumper) -> Option<(IL2CppField, u32, u32)> {
        if self.field_start.is_negative() || self.field_count == 0 {
            if let Some(struct_idx) = self.get_parent_struct_idx(meta) {
                let strukt = IL2CppStruct::from_bytes(
                    &meta.type_definitions.as_slice_of(&meta.metadata)
                    [struct_idx as usize * STRUCT_STRIDE .. struct_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                );
                if let Some((field, offset, strukt_idx)) = strukt.get_field_type_at_off(struct_idx as u32, off, meta) {
                    return Some((field, offset, strukt_idx));
                }
            }
            return None;
        }
        let mut off_vec: SmallVec<[(u32,u32);30]> = SmallVec::with_capacity(self.field_count as usize);
        let off_off_off = meta.meta_reg.field_offsets + idx as usize * 8;
        if let Some(off_off) = meta.pe.map_v2p(u64::from_le_bytes(meta.assembly[off_off_off .. off_off_off + 8].try_into().unwrap())) {
            for field_idx in 0 .. self.field_count as usize {
                let mut off = u32::from_le_bytes(meta.assembly[off_off + field_idx * 4 .. off_off + field_idx * 4 + 4].try_into().unwrap());
                let field = IL2CppField::from_bytes(
                    &meta.fields.as_slice_of(&meta.metadata)
                    [(self.field_start as usize + field_idx) * FIELD_STRIDE .. (self.field_start as usize + field_idx) * FIELD_STRIDE + FIELD_STRIDE]
                );
                let typ = &meta.types_array[field.type_idx as usize];
                if self.bitfield & 0x1 != 0 && typ.attrs & 0x10 == 0 {
                    off -= 16;
                }
                if typ.attrs & 0x50 == 0 {
                    off_vec.push((off, field_idx as u32 + self.field_start as u32));
                }
            }
        }
        if off_vec.is_empty() {
            if let Some(struct_idx) = self.get_parent_struct_idx(meta) {
                let strukt = IL2CppStruct::from_bytes(
                    &meta.type_definitions.as_slice_of(&meta.metadata)
                    [struct_idx as usize * STRUCT_STRIDE .. struct_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                );
                if let Some((field, offset, strukt_idx)) = strukt.get_field_type_at_off(struct_idx as u32, off, meta) {
                    return Some((field, offset, strukt_idx));
                }
            }
            return None;
        }
        off_vec.sort_unstable_by_key(|(x, _)| *x);
        let point = off_vec.partition_point(|(x, _)| *x <= off as u32);
        if point == 0 {
            if let Some(struct_idx) = self.get_parent_struct_idx(meta) {
                let strukt = IL2CppStruct::from_bytes(
                    &meta.type_definitions.as_slice_of(&meta.metadata)
                    [struct_idx as usize * STRUCT_STRIDE .. struct_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                );
                if let Some((field, offset, strukt_idx)) = strukt.get_field_type_at_off(struct_idx as u32, off, meta) {
                    return Some((field, offset, strukt_idx));
                }
            }
        }
        let field_idx = off_vec[max(point, 1) - 1].1 as usize;
        Some((IL2CppField::from_bytes(
            &meta.fields.as_slice_of(&meta.metadata)
            [field_idx * FIELD_STRIDE .. field_idx * FIELD_STRIDE + FIELD_STRIDE]
        ), off_vec[max(point, 1) - 1].0, idx))
    }
    
    fn get_parent_struct_idx(&self, meta: &IL2CppDumper) -> Option<u32> {
        if self.parent_idx >= 0 {
            return meta.types_array[self.parent_idx as usize].get_struct();
        }
        None
    }
    
    pub fn get_field_offsets(&self, idx: u32, meta: &IL2CppDumper, table: &mut HashMap<String, u64>) {
        self.get_field_offsets_internal(idx, 0, &meta.get_string(self.name_off), meta, table);
    }
    
    fn get_field_offsets_internal(&self, idx: u32, recursion: u32, prefix: &str, meta: &IL2CppDumper, table: &mut HashMap<String, u64>) {
        let off_off_off = meta.meta_reg.field_offsets + idx as usize * 8;
        if let Some(off_off) = meta.pe.map_v2p(u64::from_le_bytes(meta.assembly[off_off_off .. off_off_off + 8].try_into().unwrap())) {
            for field_idx in 0 .. self.field_count as usize {
                let mut off = u32::from_le_bytes(meta.assembly[off_off + field_idx * 4 .. off_off + field_idx * 4 + 4].try_into().unwrap());
                let field = IL2CppField::from_bytes(
                    &meta.fields.as_slice_of(&meta.metadata)
                    [(self.field_start as usize + field_idx) * FIELD_STRIDE .. (self.field_start as usize + field_idx) * FIELD_STRIDE + FIELD_STRIDE]
                );
                let typ = &meta.types_array[field.type_idx as usize];
                if self.bitfield & 0x1 != 0 && typ.attrs & 0x10 == 0 {
                    off = off.wrapping_sub(16);
                }
                if typ.attrs & 0x40 == 0 {
                    let name = format!("{prefix}.{}", meta.get_string(field.name_off));
                    
                    if recursion < 2 {
                        if let Some(struct_idx) = meta.types_array[field.type_idx as usize].get_struct_noref() {
                            if idx != struct_idx {
                                let strukt = IL2CppStruct::from_bytes(
                                    &meta.type_definitions.as_slice_of(&meta.metadata)
                                    [struct_idx as usize * STRUCT_STRIDE .. struct_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                                );
                                if strukt.flags & 0x2000 != 0 {
                                    strukt.get_field_offsets_internal(idx, recursion + 1, &name, meta, table);
                                }
                            }
                        }
                    }
                    table.insert(name, off as u64);
                }
            }
        }
    }
    
    fn get_enum_stuff(&self, meta: &IL2CppDumper, table: &mut HashMap<String, u64>) {
        
        fn read_compressed_u32(meta: &IL2CppDumper, off: usize) -> u32 {
            let byte = meta.metadata[off] as u32;
            if byte & 0x80 == 0 {
                byte
            } else if byte & 0x40 == 0 {
                (byte & 0x7F) << 8 | meta.metadata[off+1] as u32
            } else if byte & 0x20 == 0 {
                u32::from_be_bytes(meta.metadata[off..off+4].try_into().unwrap()) & 0x3FFF_FFFF
            } else {
                match byte {
                    0xF0 => u32::from_le_bytes(meta.metadata[off+1..off+5].try_into().unwrap()),
                    0xFE => u32::max_value() - 1,
                    0xFF => u32::max_value(),
                    other => panic!("Invalid compressed int start byte: {}", other)
                }
            }
        }
        
        fn read_compressed_i32(meta: &IL2CppDumper, off: usize) -> i32 {
            let int = read_compressed_u32(meta, off);
            if int == u32::max_value() {
                i32::min_value()
            } else if int & 0x1 == 0 {
                (int >> 1) as i32
            } else {
                -((int >> 1) as i32 + 1)
            }
        }
        
        if self.bitfield & 0x2 != 0 { //is enum
            for field_idx in 0 .. self.field_count as usize {
                let field = IL2CppField::from_bytes(
                    &meta.fields.as_slice_of(&meta.metadata)
                    [(self.field_start as usize + field_idx) * FIELD_STRIDE .. (self.field_start as usize + field_idx) * FIELD_STRIDE + FIELD_STRIDE]
                );
                let typ = &meta.types_array[field.type_idx as usize];
                if typ.attrs & 0x50 != 0 || true { //is const
                    //TODO: something
                    if let Some(field_default_idx) = meta.field_default_lookup.get(&(field_idx as i32 + self.field_start)) {
                        let il2cppdefault = IL2CppFieldDefault::from_bytes(
                            &meta.field_defaults.as_slice_of(&meta.metadata)
                            [*field_default_idx as usize * FIELD_DEFAULT_STRIDE .. *field_default_idx as usize * FIELD_DEFAULT_STRIDE + FIELD_DEFAULT_STRIDE]
                        );
                        //
                        let typ2 = &meta.types_array[il2cppdefault.type_idx as usize];
                        if il2cppdefault.data_off >= 0 {
                            let off = il2cppdefault.data_off as usize + meta.field_parameter_defaults.off as usize;
                            match typ2.kind {
                                IL2CppTypeEnum::I8 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        meta.metadata[off] as i8 as i64 as u64,
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), meta.metadata[off] as i8 as i64 as u64);
                                }
                                IL2CppTypeEnum::U8 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        meta.metadata[off] as u64,
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), meta.metadata[off] as u64);
                                }
                                IL2CppTypeEnum::I16 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        i16::from_le_bytes(meta.metadata[off..off+2].try_into().unwrap()) as i64 as u64,
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), i16::from_le_bytes(meta.metadata[off..off+2].try_into().unwrap()) as i64 as u64);
                                }
                                IL2CppTypeEnum::U16 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        u16::from_le_bytes(meta.metadata[off..off+2].try_into().unwrap()) as u64,
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), u16::from_le_bytes(meta.metadata[off..off+2].try_into().unwrap()) as u64);
                                }
                                IL2CppTypeEnum::I32 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        read_compressed_i32(meta, off) as i64 as u64,
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), read_compressed_i32(meta, off) as i64 as u64);
                                }
                                IL2CppTypeEnum::U32 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        read_compressed_u32(meta, off) as u64,
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), read_compressed_u32(meta, off) as u64);
                                }
                                IL2CppTypeEnum::I64 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        i64::from_le_bytes(meta.metadata[off..off+8].try_into().unwrap()) as u64,
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), i64::from_le_bytes(meta.metadata[off..off+8].try_into().unwrap()) as u64);
                                }
                                IL2CppTypeEnum::U64 => {
                                    table.insert(
                                        format!("{}::{}", self.get_name(meta), meta.get_string(field.name_off)),
                                        u64::from_le_bytes(meta.metadata[off..off+8].try_into().unwrap()),
                                    );
                                    //println!("{}::{} = {}", self.get_name(meta), meta.get_string(field.name_off), u64::from_le_bytes(meta.metadata[off..off+8].try_into().unwrap()));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn decode(&self, meta: &IL2CppDumper, out_str: &mut String, idx: u32) {
        let mut table: HashMap<String, u64> = HashMap::new();
        self.get_enum_stuff(meta, &mut table);
        format_to!(out_str, "struct {} {{\n", self.get_name(meta));
        let my_name = self.get_name(meta);
        //let mut field_offsets: Vec<(String, u64)> = Vec::with_capacity(self.field_count as usize);
        
        let off_off_off = meta.meta_reg.field_offsets + idx as usize * 8;
        if let Some(off_off) = meta.pe.map_v2p(u64::from_le_bytes(meta.assembly[off_off_off .. off_off_off + 8].try_into().unwrap())) {
            for field_idx in 0 .. self.field_count as usize {
                let mut off = u32::from_le_bytes(meta.assembly[off_off + field_idx * 4 .. off_off + field_idx * 4 + 4].try_into().unwrap());
                let field = IL2CppField::from_bytes(
                    &meta.fields.as_slice_of(&meta.metadata)
                    [(self.field_start as usize + field_idx) * FIELD_STRIDE .. (self.field_start as usize + field_idx) * FIELD_STRIDE + FIELD_STRIDE]
                );
                let typ = &meta.types_array[field.type_idx as usize];
                if self.bitfield & 0x1 != 0 && typ.attrs & 0x10 == 0 {
                    off = off.wrapping_sub(16);
                }
                
                if let Some(val) = table.get(&format!("{my_name}::{}", meta.get_string(field.name_off))) {
                    format_to!(out_str, "\t{}: {} = {val}, //0x{off:X}\n", meta.get_string(field.name_off), typ.name(meta).0);
                } else {
                    format_to!(out_str, "\t{}: {}, //0x{off:X}\n", meta.get_string(field.name_off), typ.name(meta).0);
                }
                
                //if typ.attrs & 0x50 == 0 {
                //    let name = format!("{prefix}.{}", );
                //    table.insert(name, off as u64);
                //}
            }
        }
        
        format_to!(out_str, "}}\n\n");
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct IL2CppField {
    name_off: u32,
    type_idx: i32,
    token:    u32,
}
const FIELD_STRIDE: usize = size_of::<IL2CppField>();
impl IL2CppField {
    fn from_bytes(data: &[u8]) -> Self {
        Self {
            name_off: u32::from_le_bytes(data[0x00..0x04].try_into().unwrap()),
            type_idx: i32::from_le_bytes(data[0x04..0x08].try_into().unwrap()),
            token:    u32::from_le_bytes(data[0x08..0x0c].try_into().unwrap()),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct IL2CppImage {
    name_off:               u32,
    assembly_idx:           i32,
    type_start:             i32,
    type_count:             u32,
    exported_type_start:    i32,
    exported_type_count:    u32,
    entry_point_idx:        i32,
    token:                  u32,
    custom_attribute_start: i32,
    custom_attribute_count: u32,
}
const IMAGE_STRIDE: usize = size_of::<IL2CppImage>();
impl IL2CppImage {
    fn from_bytes(data: &[u8]) -> Self {
        Self {
            name_off:               u32::from_le_bytes(data[0x00..0x04].try_into().unwrap()),
            assembly_idx:           i32::from_le_bytes(data[0x04..0x08].try_into().unwrap()),
            type_start:             i32::from_le_bytes(data[0x08..0x0c].try_into().unwrap()),
            type_count:             u32::from_le_bytes(data[0x0c..0x10].try_into().unwrap()),
            exported_type_start:    i32::from_le_bytes(data[0x10..0x14].try_into().unwrap()),
            exported_type_count:    u32::from_le_bytes(data[0x14..0x18].try_into().unwrap()),
            entry_point_idx:        i32::from_le_bytes(data[0x18..0x1c].try_into().unwrap()),
            token:                  u32::from_le_bytes(data[0x1c..0x20].try_into().unwrap()),
            custom_attribute_start: i32::from_le_bytes(data[0x20..0x24].try_into().unwrap()),
            custom_attribute_count: u32::from_le_bytes(data[0x24..0x28].try_into().unwrap()),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
#[repr(C)]
struct IL2CppParameter {
    name_off: u32,
    token:    u32,
    type_idx: i32,
}
const PARAMETER_STRIDE: usize = size_of::<IL2CppParameter>();
impl IL2CppParameter {
    fn from_bytes(data: &[u8]) -> Self {
        Self {
            name_off: u32::from_le_bytes(data[0x00..0x04].try_into().unwrap()),
            token:    u32::from_le_bytes(data[0x04..0x08].try_into().unwrap()),
            type_idx: i32::from_le_bytes(data[0x08..0x0c].try_into().unwrap()),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
#[repr(C)]
struct IL2CppFieldDefault {
    field_idx: i32,
    type_idx:  i32,
    data_off:  i32,
}
const FIELD_DEFAULT_STRIDE: usize = size_of::<IL2CppFieldDefault>();
impl IL2CppFieldDefault {
    fn from_bytes(data: &[u8]) -> Self {
        Self {
            field_idx: i32::from_le_bytes(data[0x00..0x04].try_into().unwrap()),
            type_idx:  i32::from_le_bytes(data[0x04..0x08].try_into().unwrap()),
            data_off:  i32::from_le_bytes(data[0x08..0x0c].try_into().unwrap()),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct IL2CppGenericParameter {
    owner_idx:         i32,
    name_off:          u32,
    constraints_start: i16,
    constraints_cnt:   i16,
    num:               u16,
    flags:             u16,
}
const GENERIC_PARAMETER_STRIDE: usize = size_of::<IL2CppGenericParameter>();
impl IL2CppGenericParameter {
    fn from_bytes(data: &[u8]) -> Self {
        Self {
            owner_idx:         i32::from_le_bytes(data[0x00..0x04].try_into().unwrap()),
            name_off:          u32::from_le_bytes(data[0x04..0x08].try_into().unwrap()),
            constraints_start: i16::from_le_bytes(data[0x08..0x0a].try_into().unwrap()),
            constraints_cnt:   i16::from_le_bytes(data[0x0a..0x0c].try_into().unwrap()),
            num:               u16::from_le_bytes(data[0x0c..0x0e].try_into().unwrap()),
            flags:             u16::from_le_bytes(data[0x0e..0x10].try_into().unwrap()),
        }
    }
}

#[derive(Clone)]
struct Icgm {
    img: IL2CppImage,
    cgm: CodeGenModule,
}

        

impl IL2CppDumper {
    pub fn initialize(path: &PathBuf) -> Result<Self, String> {
        let game_root = if path.is_file() {
            &path.parent().unwrap().to_path_buf()
        } else {
            path
        };
        let assembly = match read(game_root.join("GameAssembly.dll")) {
            Ok(bytes) => bytes,
            _ => {return Err("Failed to read GameAssembly.dll".into());}
        };
        let metadata = match read(game_root.join("PlantsVsZombiesRH_Data/il2cpp_data/Metadata/global-metadata.dat")) {
            Ok(bytes) => bytes,
            _ => {return Err("Failed to read global-metadata.dat".into());}
        };
        
        if u32::from_le_bytes(metadata[0..4].try_into().unwrap()) != 0xFAB11BAF {
            return Err("Metadata has incorrect magic number".into());
        }
        
        let version = u32::from_le_bytes(metadata[4..8].try_into().unwrap());
        
        if version != 29 {
            return Err(format!("Unsupported version: {version}"))
        }
        
        let pe = Pe::new(&assembly[..],
            ((u32::from_le_bytes(metadata[0xa4..0xa8].try_into().unwrap()) as usize)/STRUCT_STRIDE) as u64,
            u32::from_le_bytes(metadata[0xac..0xb0].try_into().unwrap())/IMAGE_STRIDE as u32
        ).expect("Invalid PE executable");
        
        let code_reg = CodeRegistration::new(&assembly, &pe);
        let meta_reg = MetadataRegistration::new(&assembly, &pe);
        //let mut acc = 0;
        //for i in 0..code_reg.code_gen_modules_cnt as usize {
        //    let code_gen_module_off = pe.map_v2p(u64::from_le_bytes(assembly[code_reg.code_gen_modules+i*8..code_reg.code_gen_modules+i*8+8].try_into().unwrap())).unwrap();
        //    let name_off = pe.map_v2p(u64::from_le_bytes(assembly[code_gen_module_off..code_gen_module_off+8].try_into().unwrap())).unwrap();
        //    let name = CStr::from_bytes_until_nul(&assembly[name_off..]).unwrap().to_string_lossy();
        //    let function_cnt = u64::from_le_bytes(assembly[code_gen_module_off+8..code_gen_module_off+16].try_into().unwrap());
        //    acc += function_cnt;
        //    println!("0x{code_gen_module_off:x} {name}: {function_cnt}");
        //}
        //println!("{acc}");
        println!("0x{:X}", pe.map_v2p(0x181BF5E98).unwrap());
        
        let mut il2cpp = Self {
            string_literals:           OffSiz::from_bytes(&metadata[0x008..0x010]),
            string_literals_data:      OffSiz::from_bytes(&metadata[0x010..0x018]),
            strings:                   OffSiz::from_bytes(&metadata[0x018..0x020]),
            events:                    OffSiz::from_bytes(&metadata[0x020..0x028]),
            properties:                OffSiz::from_bytes(&metadata[0x028..0x030]),
            methods:                   OffSiz::from_bytes(&metadata[0x030..0x038]),
            parameter_defaults:        OffSiz::from_bytes(&metadata[0x038..0x040]),
            field_defaults:            OffSiz::from_bytes(&metadata[0x040..0x048]),
            field_parameter_defaults:  OffSiz::from_bytes(&metadata[0x048..0x050]),
            field_marshaled_sizes:     OffSiz::from_bytes(&metadata[0x050..0x058]),
            parameters:                OffSiz::from_bytes(&metadata[0x058..0x060]),
            fields:                    OffSiz::from_bytes(&metadata[0x060..0x068]),
            generic_parameters:        OffSiz::from_bytes(&metadata[0x068..0x070]),
            generic_constraints:       OffSiz::from_bytes(&metadata[0x070..0x078]),
            generic_containers:        OffSiz::from_bytes(&metadata[0x078..0x080]),
            nested_types:              OffSiz::from_bytes(&metadata[0x080..0x088]),
            interfaces:                OffSiz::from_bytes(&metadata[0x088..0x090]),
            vtable_methods:            OffSiz::from_bytes(&metadata[0x090..0x098]),
            interface_offsets:         OffSiz::from_bytes(&metadata[0x098..0x0a0]),
            type_definitions:          OffSiz::from_bytes(&metadata[0x0a0..0x0a8]),
            images:                    OffSiz::from_bytes(&metadata[0x0a8..0x0b0]),
            assemblies:                OffSiz::from_bytes(&metadata[0x0b0..0x0b8]),
            field_refs:                OffSiz::from_bytes(&metadata[0x0b8..0x0c0]),
            referenced_assemblies:     OffSiz::from_bytes(&metadata[0x0c0..0x0c8]),
            attribute_data:            OffSiz::from_bytes(&metadata[0x0c8..0x0d0]),
            attribute_data_range:      OffSiz::from_bytes(&metadata[0x0d0..0x0d8]),
            uvcp_types:                OffSiz::from_bytes(&metadata[0x0d8..0x0e0]),
            uvcp_ranges:               OffSiz::from_bytes(&metadata[0x0e0..0x0e8]),
            win_runtime_type_names:    OffSiz::from_bytes(&metadata[0x0e8..0x0f0]),
            win_runtime_strings:       OffSiz::from_bytes(&metadata[0x0f0..0x0f8]),
            exported_type_definitions: OffSiz::from_bytes(&metadata[0x0f8..0x100]),
            
            methods_array:        Vec::new(),
            methods_table:        HashMap::new(),
            method_addr_table:    HashMap::new(),
            field_default_lookup: HashMap::new(),
            icgm_array:           Vec::new(),
            types_array:          Vec::new(),
            generic_class_array:  Vec::new(),
            
            code_reg,
            meta_reg,
            
            version,
            pe,
            metadata,
            assembly,
        };
        
        //for image_off in il2cpp.images.as_range().step_by(IMAGE_STRIDE) {
        //    let image = IL2CppImage::from_bytes(&il2cpp.metadata[image_off..image_off+IMAGE_STRIDE]);
        //    //println!("{}", il2cpp.get_string(image.name_off));
        //    if image.type_start >= 0 {
        //        for i in image.type_start as usize .. image.type_start as usize + image.type_count as usize {
        //            let image_off = i * STRUCT_STRIDE;
        //            let _typ = IL2CppImage::from_bytes(&il2cpp.metadata[image_off..image_off+STRUCT_STRIDE]);
        //            //println!("{}", typ.get_name())
        //        }
        //    }
        //}
        
        il2cpp.populate_icgm();
        il2cpp.populate_types();
        il2cpp.populate_methods();
        
        let mut out_str = String::new();
        il2cpp.methods_array[*il2cpp.methods_table.get("InGameUIMgr::Awake(&mut self)").unwrap() as usize].decode(&il2cpp, &mut out_str);
        
        let il2cpp_arc = Arc::new(il2cpp);
        
        match il2cpp_arc.output_disasm(concat!(env!("CARGO_MANIFEST_DIR"), "/out.s")) {
            Ok(())  => println!("Successfully output disasm"),
            Err(()) => println!("Failed to output disasm"),
        }
        
        match il2cpp_arc.output_structs(concat!(env!("CARGO_MANIFEST_DIR"), "/out_structs.rs")) {
            Ok(())  => println!("Successfully output structs"),
            Err(()) => println!("Failed to output structs"),
        }
        
        Ok(Arc::into_inner(il2cpp_arc).unwrap())
    }
    
    fn get_string<T: Into::<u64>>(&self, idx: T) -> Cow<'_,str> {
        CStr::from_bytes_until_nul(&self.strings.as_slice_of(&self.metadata)[idx.into() as usize ..]).unwrap().to_string_lossy()
    }
    
    fn get_asm_string(&self, idx: usize) -> Cow<'_,str> {
        CStr::from_bytes_until_nul(&self.assembly[idx..]).unwrap().to_string_lossy()
    }
    
    fn populate_icgm(&mut self) {
        let image_cnt = self.images.siz as usize / IMAGE_STRIDE;
        
        let mut cgm_table: HashMap<String,CodeGenModule> = HashMap::with_capacity(image_cnt);
        self.icgm_array.reserve_exact(image_cnt);
        
        for i in 0..image_cnt {
            let cgm_ptr_off = self.code_reg.code_gen_modules + i * 8;
            let cgm_off     = self.pe.map_v2p(u64::from_le_bytes(self.assembly[cgm_ptr_off..cgm_ptr_off+8].try_into().unwrap())).unwrap();
            let cgm         = CodeGenModule::from_bytes(&self.assembly[cgm_off..cgm_off+0x78], &self.pe);
            cgm_table.insert(self.get_asm_string(cgm.name_off).into_owned(), cgm);
        }
        
        for image_off in self.images.as_range().step_by(IMAGE_STRIDE) {
            let img = IL2CppImage::from_bytes(&self.metadata[image_off..image_off+IMAGE_STRIDE]);
            let cgm = cgm_table.remove(&self.get_string(img.name_off).into_owned()).unwrap();
            self.icgm_array.push(Icgm {img, cgm});
        }
    }
    
    fn populate_methods(&mut self) {
        let method_cnt = self.methods.siz as usize / METHOD_STRIDE;
        let struct_cnt = self.type_definitions.siz as usize / STRUCT_STRIDE;
        
        self.methods_array.reserve_exact(method_cnt);
        self.methods_table.reserve(method_cnt);
        let mut resolved_method_array: Vec<Option<NonZero<u16>>> = vec![None; method_cnt];
        
        let mut resolved_cgm_method_array: Vec<bool>  = vec![false; method_cnt];
        let mut resolved_cgm_idx_array:    Vec<usize> = Vec::with_capacity(self.icgm_array.len());
        
        {
            let mut acc = 0usize;
            for icgm in &self.icgm_array {
                resolved_cgm_idx_array.push(acc);
                acc += icgm.cgm.method_ptrs_cnt as usize;
            }
        }
        
        for i in 0 .. method_cnt {
            let il2cppmethod = IL2CppMethod::from_bytes(&self.methods.as_slice_of(&self.metadata)[i*METHOD_STRIDE..i*METHOD_STRIDE+METHOD_STRIDE]);
            let method = Method {
                metadata: il2cppmethod,
                addr: 0,
                len:  0,
                typ:  None
            };
            
            self.methods_array.push(method);
        }
        
        for i in 0 .. struct_cnt {
            let il2cppstruct = IL2CppStruct::from_bytes(&self.type_definitions.as_slice_of(&self.metadata)[i*STRUCT_STRIDE..i*STRUCT_STRIDE+STRUCT_STRIDE]);
            if il2cppstruct.method_start >= 0 && il2cppstruct.method_start + (il2cppstruct.method_count as i32) < method_cnt as i32 {
                for j in 0..il2cppstruct.method_count as usize {
                    self.methods_array[il2cppstruct.method_start as usize + j].typ = Some(i as u32);
                }
            }
        }
        
        for icgm in self.icgm_array.iter().enumerate() {
            if icgm.1.img.type_start >= 0 {
                for type_idx in icgm.1.img.type_start as usize .. icgm.1.img.type_start as usize + icgm.1.img.type_count as usize {
                    let il2cppstruct = IL2CppStruct::from_bytes(&self.type_definitions.as_slice_of(&self.metadata)[type_idx*STRUCT_STRIDE..type_idx*STRUCT_STRIDE+STRUCT_STRIDE]);
                    if il2cppstruct.method_start >= 0 {
                        for method_idx in il2cppstruct.method_start as usize .. il2cppstruct.method_start as usize + il2cppstruct.method_count as usize {
                            resolved_method_array[method_idx] = Some(unsafe {NonZero::new_unchecked(icgm.0 as u16 + 1)});
                            let token = (self.methods_array[method_idx].metadata.token & 0xffffff) - 1;
                            resolved_cgm_method_array[resolved_cgm_idx_array[icgm.0] + token as usize] = true;
                        }
                    }
                }
            }
        }
        
        let mut unresolved_method_table: HashMap<usize, u16> = HashMap::with_capacity(16);
        
        for icgm in self.icgm_array.iter().enumerate() {
            for i in 0 .. icgm.1.cgm.method_ptrs_cnt as usize {
                if !resolved_cgm_method_array[i + resolved_cgm_idx_array[icgm.0]] {
                    unresolved_method_table.insert(i, icgm.0 as u16);
                }
            }
        }
        
        for method_idx in 0..self.methods_array.len() {
            let method = &mut self.methods_array[method_idx];
            let token  = (method.metadata.token & 0xffffff) - 1;
            if let Some(icgm_idx) = resolved_method_array[method_idx] {
                let addr_off = self.icgm_array[icgm_idx.get() as usize - 1].cgm.method_ptrs_off + token as usize * 8;
                method.addr = u64::from_le_bytes(self.assembly[addr_off..addr_off+8].try_into().unwrap());
                //println!("{:x}", method.addr);
            } else {
                println!("a");
                if let Some(icgm_idx) = unresolved_method_table.get(&(token as usize)) {
                    let addr_off = self.icgm_array[*icgm_idx as usize].cgm.method_ptrs_off + token as usize * 8;
                    method.addr = u64::from_le_bytes(self.assembly[addr_off..addr_off+8].try_into().unwrap());
                } else {
                    println!("Failed to resolve address of {}", self.methods_array[method_idx].name(self));
                }
            }
        }
        
        {
            let mut sorted: Vec<(u64, usize)> = Vec::with_capacity(method_cnt);
            for i in self.methods_array.iter().enumerate() {
                sorted.push((i.1.addr, i.0));
            }
            sorted.sort_unstable_by_key(|(addr, _idx)| *addr);
            
            for i in 0..method_cnt-1 {
                if self.methods_array[sorted[i].1].addr != 0 {
                    self.methods_array[sorted[i].1].len = self.methods_array[sorted[i+1].1].addr - self.methods_array[sorted[i].1].addr;
                }
            }
            if self.methods_array[sorted[method_cnt-1].1].addr != 0 {
                let addr = self.methods_array[sorted[method_cnt-1].1].addr;
                let mut text_end = 0;
                for section in self.pe.sections.values() {
                    if section.vrange.as_range().contains(&((addr - self.pe.base) as usize)) {
                        text_end = (section.vrange.siz + section.vrange.off) as u64 + self.pe.base;
                    }
                }
                self.methods_array[sorted[method_cnt-1].1].len = text_end - self.methods_array[sorted[method_cnt-1].1].addr;
            }
        }
        let out_txt_path = concat!(env!("CARGO_MANIFEST_DIR"), "/out.txt");
        if let Ok(out_txt) = OpenOptions::new().create_new(true).write(true).open(out_txt_path) {
            let mut out_txt = out_txt;
            for (i, method) in self.methods_array.iter().enumerate() {
                let name = method.name(self);
                //println!("0x{:07x} 0x{:09x} 0x{:05x} {}", self.pe.map_v2p(method.1.addr).unwrap_or(0), method.1.addr, method.1.len, name);
                out_txt.write_all(format!("0x{:07x} 0x{:09x} 0x{:05x} {}\n", self.pe.map_v2p(method.addr).unwrap_or(0), method.addr, method.len, name).as_bytes()).unwrap();
                if method.addr != 0 {
                    self.method_addr_table.insert(method.addr, i as u32);
                }
                self.methods_table.insert(name, i as u32);
            }
        } else {
            for (i, method) in self.methods_array.iter().enumerate() {
                let name = method.name(self);
                if method.addr != 0 {
                    self.method_addr_table.insert(method.addr, i as u32);
                }
                self.methods_table.insert(name, i as u32);
            }
        }
    }
    
    fn populate_types(&mut self) {
        let field_default_cnt = self.field_defaults.siz as usize / FIELD_DEFAULT_STRIDE;
        self.field_default_lookup.reserve(field_default_cnt);
        self.types_array.reserve_exact(self.meta_reg.types_cnt as usize);
        self.generic_class_array.reserve_exact(self.meta_reg.generic_classes_cnt as usize);
        
        for i in 0 .. self.meta_reg.types_cnt as usize {
            let off = self.pe.map_v2p(u64::from_le_bytes(self.assembly[self.meta_reg.types+i*8..self.meta_reg.types+i*8+8].try_into().unwrap())).unwrap();
            let typ = IL2CppType::from_bytes(&self.assembly[off..off+12]);
            self.types_array.push(typ);
        }
        
        for i in 0 ..field_default_cnt {
            let il2cppdefault = IL2CppFieldDefault::from_bytes(
                &self.field_defaults.as_slice_of(&self.metadata)
                [i * FIELD_DEFAULT_STRIDE .. i * FIELD_DEFAULT_STRIDE + FIELD_DEFAULT_STRIDE]
            );
            self.field_default_lookup.insert(il2cppdefault.field_idx, i as u32);
        }
    }
    
    fn get_arg_name(&self, idx: i32) -> (String, Option<u32>) {
        //let typ = IL2CppStruct::from_bytes(&self.type_definitions.as_slice_of(&self.metadata)[(idx as usize * TYPE_STRIDE)..(idx as usize * TYPE_STRIDE)+TYPE_STRIDE]);
        //typ.get_name(&self)
        //format!("{:x}", idx)
        self.types_array[idx as usize].name(self)
    }
    
    pub fn get_field_offsets(&self, table: &mut HashMap<String, u64>) {
        let typ_cnt    = self.type_definitions.siz as usize / STRUCT_STRIDE;
        for i in 0..typ_cnt {
            let il2cppstruct = IL2CppStruct::from_bytes(&self.type_definitions.as_slice_of(&self.metadata)[i*STRUCT_STRIDE..i*STRUCT_STRIDE+STRUCT_STRIDE]);
            il2cppstruct.get_field_offsets(i as u32, self, table)
        }
    }
    
    pub fn get_enum_variants(&self, table: &mut HashMap<String, u64>) {
        let typ_cnt    = self.type_definitions.siz as usize / STRUCT_STRIDE;
        for i in 0..typ_cnt {
            let il2cppstruct = IL2CppStruct::from_bytes(&self.type_definitions.as_slice_of(&self.metadata)[i*STRUCT_STRIDE..i*STRUCT_STRIDE+STRUCT_STRIDE]);
            il2cppstruct.get_enum_stuff(self, table);
        }
    }
    
    pub fn output_disasm<T: AsRef<Path>>(self : &Arc<Self>, out_path: T) -> Result<(),()> {
        let mut sorted: Vec<u32> = Vec::with_capacity(self.methods_array.len());
        for i in 0..self.methods_array.len() {
            sorted.push(i as u32);
        }
        sorted.sort_unstable_by_key(|x| self.methods_array[*x as usize].addr);
        let sorted = Arc::new(sorted);
        
        if let Ok(out_s) = OpenOptions::new().create_new(true).truncate(true).write(true).open(out_path) {
            let mut out_s = out_s;
            
            let mut string_vec: Vec<Mutex<Option<String>>> = Vec::with_capacity(self.methods_array.len());
            for _ in 0..self.methods_array.len() {
                string_vec.push(Mutex::new(None));
            }
            let string_vec = Arc::new(string_vec);
            
            let a_bool_vec: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![false; self.methods_array.len()]));
            
            //currently, 1 thread is half a second faster for unknown reasons I will have to investigate at some point
            let thread_cnt = max(available_parallelism().map(|x| x.get()).unwrap_or(1) - 1, 1);
            let mut thread_vec = Vec::with_capacity(thread_cnt);
            for _tidx in 0..thread_cnt {
                thread_vec.push(thread::spawn({
                    let string_vec_ref = Arc::clone(&string_vec);
                    let a_bool_vec_ref = Arc::clone(&a_bool_vec);
                    let sorted_ref     = Arc::clone(&sorted);
                    let il2cpp_ref     = Arc::clone(self);
                    move || {
                        let mut current_idx = 0;
                        let mut out_str = String::new();
                        loop {
                            loop {
                                let mut bool_mutex = a_bool_vec_ref.lock().unwrap();
                                for i in current_idx..sorted_ref.len() {
                                    let taken = bool_mutex.get_mut(i).unwrap();
                                    if *taken {
                                        current_idx = i + 1;
                                    } else {
                                        *taken = true;
                                        break;
                                    }
                                }
                                break;
                            }
                            
                            if current_idx == sorted_ref.len() {
                                break;
                            }
                            
                            il2cpp_ref.methods_array[sorted_ref[current_idx] as usize].decode(&il2cpp_ref, &mut out_str);
                            
                            {
                                let mut string_mutex = string_vec_ref[current_idx].lock().unwrap();
                                *string_mutex = Some(out_str.clone());
                            }
                            
                            out_str.clear();
                        }
                    }
                }));
            }
            for t in thread_vec {
                t.join().unwrap();
            }
            
            for s in string_vec.iter() {
                out_s.write_all((*s.lock().unwrap()).as_ref().unwrap().as_bytes()).unwrap();
            }
        } else {
            return Err(());
        }
        
        Ok(())
    }
    
    pub fn output_structs<T: AsRef<Path>>(self : &Arc<Self>, out_path: T) -> Result<(),()> {
        let structs_len = self.type_definitions.siz as usize / STRUCT_STRIDE;
        
        let mut sorted: Vec<u32> = Vec::with_capacity(structs_len);
        for i in 0..structs_len {
            sorted.push(i as u32);
        }
        //sorted.sort_unstable_by_key(|x| self.methods_array[*x as usize].addr);
        let sorted = Arc::new(sorted);
        
        if let Ok(out_s) = OpenOptions::new().create_new(true).truncate(true).write(true).open(out_path) {
            let mut out_s = out_s;
            
            let mut string_vec: Vec<Mutex<Option<String>>> = Vec::with_capacity(structs_len);
            for _ in 0..structs_len {
                string_vec.push(Mutex::new(None));
            }
            let string_vec = Arc::new(string_vec);
            
            let a_bool_vec: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(vec![false; structs_len]));
            
            //currently, 1 thread is half a second faster for unknown reasons I will have to investigate at some point
            let thread_cnt = max(available_parallelism().map(|x| x.get()).unwrap_or(1) - 1, 1);
            let mut thread_vec = Vec::with_capacity(thread_cnt);
            for _tidx in 0..thread_cnt {
                thread_vec.push(thread::spawn({
                    let string_vec_ref = Arc::clone(&string_vec);
                    let a_bool_vec_ref = Arc::clone(&a_bool_vec);
                    let sorted_ref     = Arc::clone(&sorted);
                    let il2cpp_ref     = Arc::clone(self);
                    move || {
                        let mut current_idx = 0;
                        let mut out_str = String::new();
                        loop {
                            loop {
                                let mut bool_mutex = a_bool_vec_ref.lock().unwrap();
                                for i in current_idx..sorted_ref.len() {
                                    let taken = bool_mutex.get_mut(i).unwrap();
                                    if *taken {
                                        current_idx = i + 1;
                                    } else {
                                        *taken = true;
                                        break;
                                    }
                                }
                                break;
                            }
                            
                            if current_idx == sorted_ref.len() {
                                break;
                            }
                            
                            let strukt = IL2CppStruct::from_bytes(
                                &il2cpp_ref.type_definitions.as_slice_of(&il2cpp_ref.metadata)[current_idx * STRUCT_STRIDE ..current_idx * STRUCT_STRIDE + STRUCT_STRIDE]
                            );
                            strukt.decode(&il2cpp_ref, &mut out_str, current_idx as u32);
                            //il2cpp_ref.methods_array[sorted_ref[current_idx] as usize].decode(&il2cpp_ref, &mut out_str);
                            
                            {
                                let mut string_mutex = string_vec_ref[current_idx].lock().unwrap();
                                *string_mutex = Some(out_str.clone());
                            }
                            
                            out_str.clear();
                        }
                    }
                }));
            }
            for t in thread_vec {
                t.join().unwrap();
            }
            
            for s in string_vec.iter() {
                out_s.write_all((*s.lock().unwrap()).as_ref().unwrap().as_bytes()).unwrap();
            }
        } else {
            return Err(());
        }
        
        Ok(())
    }
}
