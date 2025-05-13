use core::str;
use std::{arch::x86_64::{__m128i, _mm_and_si128, _mm_loadl_epi64, _mm_set1_epi8, _mm_setr_epi8, _mm_shuffle_epi8, _mm_srli_epi64, _mm_storeu_si128, _mm_unpacklo_epi8}, cmp::{max, min}, collections::HashMap, hash::BuildHasherDefault};
use fxhash::FxHashMap;
use iced_x86::{CC_ae, CC_b, CC_be, CC_np, CC_p, Code, Decoder, DecoderOptions, FlowControl, Formatter, GasFormatter, Instruction, MemorySize, Mnemonic, OpKind, Register, SymbolResolver, SymbolResult};
//use smallvec::SmallVec;

use crate::format_to;

use super::{IL2CppDumper, IL2CppParameter, IL2CppStruct, Method, PARAMETER_STRIDE, STRUCT_STRIDE};


impl Method {
    pub fn decode(&self, meta: &IL2CppDumper, out_str: &mut String) {
        let start_off = if let Some(start_off) = meta.pe.map_v2p(self.addr) {
            start_off
        } else {
            return;
        };
        let mut label_name    = String::from("\"") + &self.name_short(meta);
        let function_name_len = label_name.len();
        
        let decoder = Decoder::with_ip(
            64,
            &meta.assembly[start_off .. start_off + self.len as usize],
            self.addr,
            DecoderOptions::NO_INVALID_CHECK
        );
        let instructions: Vec<Instruction> = decoder.into_iter().collect();
        let mut mnemonic_len_vec:  Vec<u8> = Vec::with_capacity(instructions.len());
        let mut operand_len_vec:   Vec<u8> = Vec::with_capacity(instructions.len());
        let mut indent_vec:        Vec<u8> = vec![1; instructions.len()];
        
        let mut mnemonic           = String::new();
        let mut mnemonic_formatter = GasFormatter::new();
        set_formatter_options(&mut mnemonic_formatter);
        
        let (locs, loops, unknown_calls, last_ret_idx) = self.get_locs_loops_calls_endret(&instructions, meta);
        
        {
            let mut unknown_call_vec: Vec<(u64, usize)> = Vec::with_capacity(unknown_calls.len());
            
            for (addr, idx) in &unknown_calls {
                unknown_call_vec.push((*addr, *idx));
            }
            
            unknown_call_vec.sort_unstable_by_key(|(_addr, idx)| {*idx});
            
            for (addr, idx) in unknown_call_vec {
                format_label(idx, &mut label_name, function_name_len, ".unknown_call");
                format_to!(out_str, ".def    {label_name}, 0x{addr:x}\n.global {label_name}\n");
            }
        }
        
        for (i, instruction) in instructions.iter().enumerate() {
            match instruction.flow_control() {
                FlowControl::UnconditionalBranch |
                FlowControl::ConditionalBranch => {
                    let addr = instruction.near_branch64();
                    if addr >= self.addr && addr <= self.addr + self.len {
                        if instruction.next_ip() <= addr {
                            let mut indent  = true;
                            let mut last    = false;
                            let mut end_idx = i+1;
                            for (j, instruction_b) in instructions[i+1..].iter().enumerate() {
                                if instruction_b.next_ip() == addr {
                                    end_idx += j + 1;
                                    last = true;
                                }
                                match instruction_b.flow_control() {
                                    FlowControl::UnconditionalBranch |
                                    FlowControl::ConditionalBranch => {
                                        let addr_b = instruction_b.near_branch64();
                                        if addr_b >= self.addr && addr_b <= self.addr + self.len {
                                            if (addr_b >= addr || addr_b <= instruction.next_ip()) && last {
                                                end_idx -= 1;
                                            }
                                        } else if instruction_b.mnemonic() == Mnemonic::Jmp {
                                            indent = false;
                                            break;
                                        }
                                    }
                                    FlowControl::Return |
                                    FlowControl::Interrupt => {
                                        indent = false;
                                        break;
                                    }
                                    _ => {}
                                }
                                if last {
                                    break;
                                }
                            }
                            if indent {
                                for i in indent_vec[i+1..end_idx].iter_mut() {
                                    *i = i.saturating_add(1);
                                }
                            }
                        } else {
                            let mut indent    = true;
                            let mut start_idx = 0;
                            for (i, instruction) in instructions[..i].iter().rev().enumerate() {
                                if instruction.ip() == addr {
                                    start_idx += i + 1;
                                    break;
                                }
                                match instruction.flow_control() {
                                    FlowControl::Return |
                                    FlowControl::Interrupt => {
                                        indent = false;
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            if indent {
                                for i in indent_vec[i-start_idx..i].iter_mut() {
                                    *i += 1;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            mnemonic_formatter.format_mnemonic_options(instruction, &mut mnemonic, 0u32);
            mnemonic_len_vec.push(mnemonic.len() as u8 + 1);
            mnemonic.drain(..);
        }
        let full_name = self.name(meta);
        format_to!(out_str, "\n.global \"{full_name}\"\n\"{full_name}\":\n");
        
        if !instructions.is_empty() {
            let mut reg_data    = RegData::from_method(self, meta);
            let mut out_string  = String::new();
            let symbol_resolver = Box::new(DecodeSymbolResolver::new(
                locs.clone(),
                loops.clone(),
                unknown_calls,
                meta,
                label_name.clone(),
                function_name_len,
                &mut reg_data
            ));
            let mut formatter = GasFormatter::with_options(Some(symbol_resolver), None);
            set_formatter_options(&mut formatter);
            
            for instruction in &instructions[0..last_ret_idx+1] {
                formatter.format_all_operands(instruction, &mut out_string);
                operand_len_vec.push(min(out_string.len() + max(1, instruction.op_count() as usize) - 1, 255) as u8);
                out_string.drain(..);
                reg_data.process_instruction(instruction, meta);
            }
            
            reg_data = RegData::from_method(self, meta);
            
            let mut last_indent_change = 0;
            for i in 1..last_ret_idx+1 {
                if indent_vec[i-1] != indent_vec[i] {
                    let mut mnemonic_maximum = 0;
                    let mut operand_maximum  = 0;
                    for j in last_indent_change..i {
                        mnemonic_maximum = max(mnemonic_maximum, mnemonic_len_vec[j]);
                        if operand_len_vec[j] <= 24 {
                            operand_maximum  = max(operand_maximum,  operand_len_vec[j]);
                        }
                    }
                    for j in last_indent_change..i {
                        mnemonic_len_vec[j] = mnemonic_maximum;
                        operand_len_vec[j]  = operand_maximum - min(operand_maximum, operand_len_vec[j]) + max(instructions[j].op_count() as u8, 1) - 1;
                    }
                    last_indent_change = i;
                }
            }
            
            let mut mnemonic_maximum = 0;
            let mut operand_maximum  = 0;
            for j in last_indent_change..last_ret_idx+1 {
                mnemonic_maximum = max(mnemonic_maximum, mnemonic_len_vec[j]);
                if operand_len_vec[j] <= 24 {
                    operand_maximum  = max(operand_maximum,  operand_len_vec[j]);
                }
            }
            for j in last_indent_change..last_ret_idx+1 {
                mnemonic_len_vec[j] = mnemonic_maximum;
                operand_len_vec[j]  = operand_maximum - min(operand_maximum, operand_len_vec[j]) + max(instructions[j].op_count() as u8, 1) - 1;
            }
            
            for (i, instruction) in instructions[0..last_ret_idx+1].iter().enumerate() {
                let label_ident = if i != 0 {
                    min(indent_vec[i-1], indent_vec[i]) as usize
                } else {
                    1
                };
                if let Some(idx) = locs.get(&instruction.ip()) {
                    format_label(*idx, &mut label_name, function_name_len, ".loc");
                    format_to!(out_str, "{}{}: #0x{:X}\n", "\t".repeat(label_ident), &label_name, instruction.ip());
                }
                if let Some(idx) = loops.get(&instruction.ip()) {
                    format_label(*idx, &mut label_name, function_name_len, ".loop");
                    format_to!(out_str, "{}{}: #0x{:X}\n", "\t".repeat(label_ident), &label_name, instruction.ip());
                }
                let tabs = indent_vec[i] as isize;
                formatter
                    .options_mut()
                    .set_first_operand_char_index(mnemonic_len_vec[i] as u32);
                formatter.format(instruction, &mut out_string);
                
                if instruction.op_count() > 1 {
                    let spaces       = " ".repeat(((operand_len_vec[i] as u32 + instruction.op_count() - 2) / (instruction.op_count() - 1)) as usize);
                    let mut counter  = instruction.op_count() as usize - (spaces.len() * (instruction.op_count() as usize - 1) - operand_len_vec[i] as usize);
                    let mut inquotes = false;
                    let mut inparens = false;
                    let mut str_pos  = out_string.chars().count();
                    for c in out_string.clone().chars().rev() {
                        match c {
                            '"' => {inquotes = !inquotes}
                            ')' => {inparens = true}
                            '(' => {inparens = false}
                            ',' => {
                                if !(inquotes || inparens) {
                                    if counter > 0 {
                                        out_string.insert_str(str_pos, &spaces);
                                        counter -= 1;
                                    } else {
                                        out_string.insert_str(str_pos, &spaces[1..]);
                                    }
                                }
                            }
                            _ => {}
                        }
                        str_pos -= 1
                    }
                }
                
                for _ in 0..tabs {
                    out_str.push('\t');
                }
                out_str.push_str(&out_string);
                out_str.push_str(" #0x");
                let mut out_buf = [0u8; 16];
                out_str.push_str(unsafe {
                    let nibble_mask = _mm_set1_epi8(0xf);
                    let hex_digits = _mm_setr_epi8(
                        b'0' as i8, b'1' as i8, b'2' as i8, b'3' as i8, b'4' as i8, b'5' as i8, b'6' as i8,
                        b'7' as i8, b'8' as i8, b'9' as i8, b'A' as i8, b'B' as i8, b'C' as i8, b'D' as i8,
                        b'E' as i8, b'F' as i8,
                    );
                    let v  = _mm_loadl_epi64(&instruction.ip().to_be_bytes() as *const [u8] as *const __m128i);
                    let v4 = _mm_srli_epi64(v, 4);
                    let il = _mm_unpacklo_epi8(v4, v);
                    let m  = _mm_and_si128(il, nibble_mask);
                    let hexchars = _mm_shuffle_epi8(hex_digits, m);
                    let pad = (instruction.ip() | 0x1).leading_zeros() as usize >> 2;
                    _mm_storeu_si128(&mut out_buf as *mut [u8] as *mut __m128i, hexchars);

                    str::from_utf8(&out_buf[pad..]).unwrap()
                }); //thanks nick for finding a solution to this that is better than what I was using
                //and that the compiler doesn't break with its "optimizations" that add an extra like 5 instructions
                out_str.push('\n');
                out_string.clear();
                reg_data.process_instruction(instruction, meta);
            }
            
            *out_str += "\n";
        }
    }
    
    #[allow(clippy::type_complexity)]
    pub fn get_locs_loops_calls_endret(&self, instructions: &[Instruction], meta: &IL2CppDumper) -> (FxHashMap<u64, usize>, FxHashMap<u64, usize>, FxHashMap<u64, usize>, usize) {
        let mut locs:          FxHashMap<u64, usize> = HashMap::with_capacity_and_hasher(32, BuildHasherDefault::default());
        let mut loops:         FxHashMap<u64, usize> = HashMap::with_capacity_and_hasher(32, BuildHasherDefault::default());
        let mut unknown_calls: FxHashMap<u64, usize> = HashMap::with_capacity_and_hasher(32, BuildHasherDefault::default());
        
        let mut last_ret_idx = None;
        let mut last_loc_ip  = 0u64;
        for (i, instruction) in instructions.iter().enumerate() {
            match instruction.flow_control() {
                FlowControl::UnconditionalBranch |
                FlowControl::ConditionalBranch => {
                    let addr = instruction.near_branch64();
                    if addr >= self.addr && addr <= self.addr + self.len {
                        if instruction.next_ip() <= addr {
                            if !locs.contains_key(&addr) {
                                last_loc_ip  = max(addr, last_loc_ip);
                                locs.insert(addr, locs.len());
                            }
                        } else if !loops.contains_key(&addr) {
                            loops.insert(addr, loops.len());
                        }
                    } else if instruction.mnemonic() == Mnemonic::Jmp && last_loc_ip <= instruction.ip(){
                        last_ret_idx = Some(i);
                        break;
                    }
                }
                FlowControl::Call => {
                    let addr = instruction.near_branch64();
                    if !meta.method_addr_table.contains_key(&addr) && !unknown_calls.contains_key(&addr) {
                        let len = unknown_calls.len();
                        unknown_calls.insert(addr, len);
                    }
                }
                FlowControl::Return |
                FlowControl::Interrupt => {
                    if last_loc_ip <= instruction.ip() {
                        last_ret_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        
        (locs, loops, unknown_calls, last_ret_idx.unwrap_or(max(instructions.len(),1)-1))
    }
    
    pub fn get_local_syms(&self, off: i64, meta: &IL2CppDumper) -> FxHashMap<String, u64> {
        let mut ret   = HashMap::with_capacity_and_hasher(48, BuildHasherDefault::default());
        let start_off = meta.pe.map_v2p(self.addr).unwrap();
        let decoder   = Decoder::with_ip(
            64,
            &meta.assembly[start_off .. start_off + self.len as usize],
            self.addr,
            DecoderOptions::NO_INVALID_CHECK
        );
        let instructions: Vec<Instruction> = decoder.into_iter().collect();
        
        let mut label_name    = String::from("\"") + &self.name_short(meta);
        let function_name_len = label_name.len();
        
        let (locs, loops, _calls, _endret) = self.get_locs_loops_calls_endret(&instructions, meta);
        
        for (addr, idx) in locs {
            format_label(idx, &mut label_name, function_name_len, ".loc");
            ret.insert(label_name.clone(), (addr as i64 + off) as u64);
        }
        
        for (addr, idx) in loops {
            format_label(idx, &mut label_name, function_name_len, ".loop");
            ret.insert(label_name.clone(), (addr as i64 + off) as u64);
        }
        
        ret.shrink_to_fit();
        ret
    }
    
    pub fn get_calls(&self, meta: &IL2CppDumper, off: i64, table: &mut FxHashMap<String, u64>) {
        let start_off = meta.pe.map_v2p(self.addr).unwrap();
        let decoder   = Decoder::with_ip(
            64,
            &meta.assembly[start_off .. start_off + self.len as usize],
            self.addr,
            DecoderOptions::NO_INVALID_CHECK
        );
        let instructions: Vec<Instruction> = decoder.into_iter().collect();
        
        let mut label_name    = self.name_short(meta);
        let function_name_len = label_name.len();
        
        let (_locs, _loops, calls, _endret) = self.get_locs_loops_calls_endret(&instructions, meta);
        
        for (addr, idx) in calls {
            format_label(idx, &mut label_name, function_name_len, ".unknown_call");
            table.insert(label_name.clone(), (addr as i64 + off) as u64);
        }
    }
}

#[derive(Clone, Copy)]
enum RegContents {
    None,
    Strukt(u32),
    StruktStatic(u32),
    B8Strukt(u32),
}
impl From<Option<u32>> for RegContents {
    fn from(value: Option<u32>) -> Self {
        match value {
            None => RegContents::None,
            Some(x) => RegContents::Strukt(x),
        }
    }
}

struct RegData {
    gprs:  [RegContents;16],
    stack: FxHashMap<i32, u32>,
    sp:    i32,
}
impl RegData {
    fn from_method(method: &Method, meta: &IL2CppDumper) -> Self {
        let mut parameter_number = 0;
        
        let mut ret = Self {
            gprs:  [RegContents::None; 16],
            stack: HashMap::with_capacity_and_hasher(32, BuildHasherDefault::default()),
            sp:    0,
        };
        
        if !method.is_static() {
            ret.push_arg(method.typ.into(), &mut parameter_number);
        }
        
        if method.metadata.parameter_start >= 0 {
            for i in method.metadata.parameter_start as usize .. method.metadata.parameter_start as usize + method.metadata.parameter_count as usize {
                let arg = IL2CppParameter::from_bytes(&meta.parameters.as_slice_of(&meta.metadata)[i*PARAMETER_STRIDE..i*PARAMETER_STRIDE+PARAMETER_STRIDE]);
                if arg.type_idx >= 0 {
                    let typ = &meta.types_array[arg.type_idx as usize];
                    ret.push_arg(typ.get_struct().into(), &mut parameter_number);
                } else {
                    ret.push_arg(RegContents::None, &mut parameter_number);
                }
            }
        }
        
        ret
    }
    
    fn push_arg(&mut self, typ: RegContents, number: &mut i32) {
        match *number {
            0 => {self.gprs[1] = typ}
            1 => {self.gprs[2] = typ}
            2 => {self.gprs[8] = typ}
            3 => {self.gprs[9] = typ}
            _ => {
                if let RegContents::Strukt(typ) = typ {
                    self.stack.insert((*number + 1) * -8, typ);
                }
            }
        }
        *number += 1;
    }
    
    fn process_instruction(&mut self, instruction: &Instruction, meta: &IL2CppDumper) {
        match instruction.code() {
            Code::Mov_rm64_r64 |
            Code::Mov_r64_rm64 => {
                let src = self.get_src_operand(instruction, meta);
                match instruction.op0_kind() {
                    OpKind::Register => {
                        self.gprs[instruction.op0_register().number()] = src;
                    }
                    OpKind::Memory => {
                        if let Register::RSP = instruction.memory_base() {
                            let offset = instruction.memory_displacement32() as i32 + self.sp;
                            match src {
                                RegContents::Strukt(x) => {self.stack.insert(offset, x);},
                                _ => {self.stack.remove(&offset);}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Code::Add_rm64_imm32 |
            Code::Add_rm64_imm8 => {
                if let OpKind::Register = instruction.op0_kind() {
                    if let Register::RSP = instruction.op0_register() {
                        self.sp += instruction.immediate(1) as i32;
                    }
                }
            }
            Code::Sub_rm64_imm32 |
            Code::Sub_rm64_imm8 => {
                if let OpKind::Register = instruction.op0_kind() {
                    if let Register::RSP = instruction.op0_register() {
                        self.sp -= instruction.immediate(1) as i32;
                    }
                }
            }
            Code::Call_rel32_64 => {
                if let Some(idx) = meta.method_addr_table.get(&instruction.near_branch64()) {
                    let method = &meta.methods_array[*idx as usize];
                    if method.metadata.return_type >= 0 {
                        let typ = &meta.types_array[method.metadata.return_type as usize];
                        self.gprs[0] = typ.get_struct().into();
                    } else {
                        self.gprs[0] = RegContents::None
                    }
                } else {
                    self.gprs[0] = RegContents::None
                }
                self.gprs[1]  = RegContents::None;
                self.gprs[2]  = RegContents::None;
                self.gprs[8]  = RegContents::None;
                self.gprs[9]  = RegContents::None;
                self.gprs[10] = RegContents::None;
                self.gprs[11] = RegContents::None;
            }
            Code::Movzx_r64_rm8  |
            Code::Movzx_r32_rm8  |
            Code::Movzx_r16_rm8  |
            Code::Movzx_r64_rm16 |
            Code::Movzx_r32_rm16 |
            Code::Mov_r8_rm8     |
            Code::Mov_r16_rm16   |
            Code::Mov_r32_rm32   |
            Code::Mov_r8_imm8    |
            Code::Mov_r16_imm16  |
            Code::Mov_r32_imm32  |
            Code::Mov_r64_imm64 => {
                self.gprs[instruction.op0_register().full_register().number()] = RegContents::None;
            }
            _ => {}
        }
        if instruction.is_stack_instruction() {
            match instruction.mnemonic() {
                Mnemonic::Call => {}
                _ => self.sp += instruction.stack_pointer_increment()
            }
        }
    }

    fn get_src_operand(&mut self, instruction: &Instruction, meta: &IL2CppDumper) -> RegContents {
        match instruction.op1_kind() {
            OpKind::Register => {
                self.gprs[instruction.op1_register().number()]
            }
            OpKind::Memory => {
                match instruction.memory_base() {
                    Register::RSP => {
                        self.stack.get(&(instruction.memory_displacement32() as i32 + self.sp)).cloned().into()
                    }
                    Register::None => RegContents::None,
                    Register::RIP => match instruction.memory_size() {
                        MemorySize::UInt64 |
                        MemorySize::Int64 => {
                            if let Some(off) = meta.pe.map_v2p(instruction.ip_rel_memory_address()) {
                                let qword = u64::from_le_bytes(meta.assembly[off .. off + 8].try_into().unwrap());
                                if qword >> 32 != 0 || qword & 1 == 0 {
                                    return RegContents::None;
                                }
                                if qword >> 29 != 1 {
                                    return RegContents::None;
                                }
                                let type_idx = ((qword >> 1) & 0xFFF_FFFF) as usize;
                                
                                if let Some(typ) = meta.types_array.get(type_idx) {
                                    match typ.get_struct() {
                                        None => RegContents::None,
                                        Some(x) => RegContents::B8Strukt(x),
                                    }
                                } else {
                                    RegContents::None
                                }
                            } else {
                                RegContents::None
                            }
                        }
                        _ => RegContents::None
                    },
                    _ => {
                        let contents = self.gprs[instruction.memory_base().number()];
                        match contents {
                            RegContents::StruktStatic(struct_idx) |
                            RegContents::Strukt(struct_idx) => {
                                let strukt = IL2CppStruct::from_bytes(
                                    &meta.type_definitions.as_slice_of(&meta.metadata)
                                    [struct_idx as usize * STRUCT_STRIDE .. struct_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                                );
                                if let Some((field, field_off, _)) = strukt.get_field_type_at_off(
                                    struct_idx,
                                    instruction.memory_displacement64(),
                                    matches!(contents, RegContents::StruktStatic(_)),
                                    meta
                                ) {
                                    if field_off == instruction.memory_displacement32() {
                                        if let Some(strukt_idx) = meta.types_array[field.type_idx as usize].get_struct() {
                                            let strukt = IL2CppStruct::from_bytes(
                                                &meta.type_definitions.as_slice_of(&meta.metadata)
                                                [struct_idx as usize * STRUCT_STRIDE .. struct_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                                            );
                                            if strukt.flags & 0x2000 == 0 {
                                                RegContents::Strukt(strukt_idx)
                                            } else {
                                                RegContents::None
                                            }
                                        } else {
                                            RegContents::None
                                        }
                                    } else {
                                        RegContents::None
                                    }
                                } else {
                                    RegContents::None
                                }
                            },
                            RegContents::B8Strukt(idx) => {
                                if instruction.memory_displacement32() == 0xB8 {
                                    RegContents::StruktStatic(idx)
                                } else {
                                    RegContents::None
                                }
                            }
                            RegContents::None => RegContents::None,
                        }
                    }
                }
            }
            _ => RegContents::None
        }
    }
}
struct DecodeSymbolResolver {
    locs:          FxHashMap<u64, usize>,
    loops:         FxHashMap<u64, usize>,
    unknown_calls: FxHashMap<u64, usize>,
    label_name:                   String,
    function_name_len:             usize,
    meta:            *const IL2CppDumper,
    reg_data:               *mut RegData,
}
impl DecodeSymbolResolver {
    fn new(locs: FxHashMap<u64, usize>, loops: FxHashMap<u64, usize>, unknown_calls: FxHashMap<u64, usize>, meta: &IL2CppDumper,
        label_name: String, function_name_len: usize, reg_data: &mut RegData) -> Self {
        Self {
            locs,
            loops,
            unknown_calls,
            function_name_len,
            label_name,
            reg_data: reg_data as *mut RegData,
            meta: meta as *const IL2CppDumper,
        }
    }
}
impl SymbolResolver for DecodeSymbolResolver {
    fn symbol(&mut self, instruction: &Instruction, _operand: u32, instruction_operand: Option<u32>, addr: u64, _addr_size: u32) -> Option<SymbolResult> {
        if instruction.next_ip() <= addr {
            if let Some(idx) = self.locs.get(&addr) {
                format_label(*idx, &mut self.label_name, self.function_name_len, ".loc");
                return Some(SymbolResult::with_str(addr, &self.label_name));
            }
        } else if let Some(idx) = self.loops.get(&addr) {
            format_label(*idx, &mut self.label_name, self.function_name_len, ".loop");
            return Some(SymbolResult::with_str(addr, &self.label_name));
        }
        if let Some(idx) = self.unknown_calls.get(&addr) {
            format_label(*idx, &mut self.label_name, self.function_name_len, ".unknown_call");
            return Some(SymbolResult::with_str(addr, &self.label_name));
        }
        let meta = unsafe {&*self.meta as &IL2CppDumper};
        
        if let Some(idx) = meta.method_addr_table.get(&addr) {
            let func_name = format!("\"{}\"", meta.methods_array[*idx as usize].name(meta));
            return Some(SymbolResult::with_string(addr, func_name));
        }
        //if let Some(off) = meta.pe.map_v2p_data(addr) {
        //    return Some(SymbolResult::with_string(addr,format!("GameAssembly.dll+0x{off:x}")))
        //}
        if let Some(off) = meta.pe.map_v2p_data(addr) {
            match instruction.mnemonic() {
                Mnemonic::Addss   |
                Mnemonic::Subss   |
                Mnemonic::Mulss   |
                Mnemonic::Divss   |
                Mnemonic::Movss   |
                Mnemonic::Comiss  |
                Mnemonic::Ucomiss |
                Mnemonic::Sqrtss  |
                Mnemonic::Cmpss   |
                Mnemonic::Maxss   |
                Mnemonic::Minss   |
                Mnemonic::Rcpss   |
                Mnemonic::Roundss |
                Mnemonic::Rsqrtss => {
                    let fpval = f32::from_le_bytes(meta.assembly[off..off+4].try_into().unwrap());
                    return Some(SymbolResult::with_string(addr, format!("const{}{}", if fpval.is_sign_positive() {""} else {"m"}, fpval.abs())));
                }
                _ => {}
            }
        }
        
        let reg_data = unsafe {&*self.reg_data as &RegData};
        
        if let Some(operand) = instruction_operand {
            if let OpKind::Memory = instruction.op_kind(operand) {
                match instruction.memory_base() {
                    Register::None => {}
                    Register::RIP  => {}
                    _ => match reg_data.gprs[instruction.memory_base().number()] {
                        RegContents::StruktStatic(class_idx) |
                        RegContents::Strukt(class_idx) => {
                            let strukt = IL2CppStruct::from_bytes(
                                &meta.type_definitions.as_slice_of(&meta.metadata)
                                [class_idx as usize * STRUCT_STRIDE .. class_idx as usize * STRUCT_STRIDE + STRUCT_STRIDE]
                            );
                            let string = format!("{}.{}", meta.get_string(strukt.name_off), strukt.get_field_name_at_off(
                                class_idx,
                                addr,
                                matches!(reg_data.gprs[instruction.memory_base().number()], RegContents::StruktStatic(_)),
                                meta,
                            ));
                            if !string.is_empty() {
                                return Some(SymbolResult::with_string(addr, string))
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        None
    }
}

fn format_label(number: usize, label_name: &mut String, function_name_len: usize, kind: &str) {
    let mut tmp_idx = number;
    label_name.replace_range(function_name_len.., kind);
    
    label_name.insert(function_name_len + kind.len(), char::from((0x41+(tmp_idx%26)) as u8));
    tmp_idx /= 26;
    while tmp_idx != 0 {
        label_name.insert(function_name_len + kind.len(), char::from((0x41+(tmp_idx%26)) as u8));
        tmp_idx /= 26;
    }
    
    if label_name.starts_with('"') {
        label_name.push('"');
    }
}

fn set_formatter_options(formatter: &mut GasFormatter) {
    formatter
        .options_mut()
        .set_first_operand_char_index(8);
    formatter
        .options_mut()
        .set_leading_zeros(false);
    formatter
        .options_mut()
        .set_small_hex_numbers_in_decimal(false);
    formatter
        .options_mut()
        .set_add_leading_zero_to_hex_numbers(false);
    formatter
        .options_mut()
        .set_signed_immediate_operands(true);
    formatter
        .options_mut()
        .set_branch_leading_zeros(false);
    formatter
        .options_mut()
        .set_prefer_st0(true);
    formatter
        .options_mut()
        .set_cc_b(CC_b::c);
    formatter
        .options_mut()
        .set_cc_ae(CC_ae::nc);
    formatter
        .options_mut()
        .set_cc_be(CC_be::na);
    formatter
        .options_mut()
        .set_cc_p(CC_p::pe);
    formatter
        .options_mut()
        .set_cc_np(CC_np::po);
}
