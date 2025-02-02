use std::{collections::HashMap, io::Write, num::NonZeroU64, ops::Range};

use iced_x86::{Code, Decoder, DecoderOptions, Encoder, Formatter, GasFormatter, Instruction, Mnemonic, OpKind};
use object::{File, Object, ObjectSection, ObjectSymbol, Relocation, RelocationKind, RelocationTarget, Section, SectionKind, Symbol, SymbolKind};
use smallvec::SmallVec;

use crate::{process::{FusionProcess, PAGE_EXECUTE_READWRITE}, util::CommonError};

use super::il2cppdump::IL2CppDumper;
struct TextSection {
    _idx:                       usize,
    instructions:   Vec<Instruction>,
    relocs: HashMap<u64, Relocation>,
}

struct DataSection {
    _idx:    usize,
    relocs: HashMap<u64, Relocation>,
    range:  Range<usize>,
}
#[allow(dead_code)]
#[derive(Debug)]
#[derive(Clone)]
pub struct Patch {
    injections:   SmallVec<[Injection; 16]>,
    instructions: Vec<Instruction>,
    imm_vec:      Vec<Immediate>,
    data:         Option<Vec<u8>>,
    data_relocs:  Vec<DataReloc>,
    patch_syms:   HashMap<String, PatchSymbolLocation>,
}
impl Patch {
    pub fn new(obj: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::parse(obj)?;
        let sections: Vec<Section> = file.sections().collect();
        let mut text_section: Option<TextSection> = None;
        let mut data_section: Option<DataSection> = None;
        
        for (i, section) in sections.iter().enumerate() {
            match section.kind() {
                SectionKind::Text => {
                    if let Some((start, size)) = section.file_range() {
                        let start = start as usize;
                        let size  = size  as usize;
                        let end   = start + size;
                        
                        let decoder = Decoder::with_ip(
                            64,
                            &obj[start..end],
                            0x40_0000,
                            DecoderOptions::NO_INVALID_CHECK
                        );
                        let instructions: Vec<Instruction> = decoder.into_iter().collect();
                        text_section = Some(TextSection {
                            _idx: i,
                            instructions,
                            relocs: section.relocations().collect(),
                        });
                    }
                }
                SectionKind::Data => {
                    if let Some((start, size)) = section.file_range() {
                        data_section = Some(DataSection {
                            _idx: i,
                            relocs: section.relocations().collect(),
                            range: start as usize .. start as usize + size as usize,
                        });
                    }
                }
                _ => {}
            }
        }
        
        let mut text_section = text_section.ok_or(CommonError::critical("No text section found in patch object file"))?;
        
        let mut sym_array: Vec<(usize, Symbol)> = file.symbols().enumerate().collect();
        for (i, sym) in sym_array.iter_mut() {
            *i = sym.index().0;
        }
        let symbols: HashMap<usize, Symbol> = sym_array.into_iter().collect();
        
        let mut imm_vec: Vec<Immediate> = Vec::with_capacity(256);
        let mut rev_imm_lookup: SmallVec<[usize;256]> = SmallVec::new();
        let mut branch_target_table: HashMap<u64, usize> = HashMap::with_capacity(text_section.instructions.len());
        
        for (i, instruction) in text_section.instructions.iter().enumerate() {
            branch_target_table.insert(instruction.ip(), i);
        }
        
        let mut near_branches: SmallVec<[(u32, u32); 32]>  = SmallVec::new();
        let mut imms:          SmallVec<[(u32, u32); 128]> = SmallVec::new();
        let mut mem_offsets:   SmallVec<[(u32, u32); 128]> = SmallVec::new();
        
        for instruction_idx in 0..text_section.instructions.len() {
            let instruction = &text_section.instructions[instruction_idx];
            for (op_idx, op_kind) in instruction.op_kinds().enumerate() {
                let idx = imm_vec.len();
                let (imm_off, mem_off) = get_instruction_imm_and_memory_offsets(instruction);
                match op_kind {
                    OpKind::NearBranch64 => {
                        if let Some(branch_instruction_idx) = branch_target_table.get(&instruction.near_branch64()) {
                            if let Some(_reloc) = text_section.relocs.get(&(instruction.ip() + imm_off - 0x40_0000)) {
                                rev_imm_lookup.push(instruction_idx);
                                imm_vec.push(reloc_to_immediate(
                                    text_section.relocs.get(&(instruction.ip() + imm_off - 0x40_0000)).unwrap(),
                                    &symbols,
                                    &branch_target_table,
                                    &sections,
                                    instruction.near_branch64() as i64 - instruction.next_ip() as i64,
                                    match instruction.mnemonic() {
                                        Mnemonic::Call => true,
                                        _ => false
                                    },
                                ));
                            } else {
                                rev_imm_lookup.push(instruction_idx);
                                match instruction.mnemonic() {
                                    Mnemonic::Call => imm_vec.push(Immediate::InstructionOffsetCall(*branch_instruction_idx)),
                                    _              => imm_vec.push(Immediate::InstructionOffset(*branch_instruction_idx))
                                }
                            }
                        } else {
                            rev_imm_lookup.push(instruction_idx);
                            imm_vec.push(reloc_to_immediate(
                                text_section.relocs.get(&(instruction.ip() + imm_off - 0x40_0000)).unwrap(),
                                &symbols,
                                &branch_target_table,
                                &sections,
                                instruction.near_branch64() as i64 - instruction.next_ip() as i64,
                                match instruction.mnemonic() {
                                    Mnemonic::Call => true,
                                    _ => false
                                },
                            ));
                        }
                        near_branches.push((instruction_idx as u32, idx as u32));
                    }
                    OpKind::Immediate8 |
                    OpKind::Immediate16 |
                    OpKind::Immediate32 |
                    OpKind::Immediate64 |
                    OpKind::Immediate8to16 |
                    OpKind::Immediate8to32 |
                    OpKind::Immediate8to64 |
                    OpKind::Immediate32to64 => {
                        if let Some(reloc) = text_section.relocs.get(&(instruction.ip() + imm_off - 0x40_0000)) {
                            rev_imm_lookup.push(instruction_idx);
                            imm_vec.push(reloc_to_immediate(
                                reloc,
                                &symbols,
                                &branch_target_table,
                                &sections,
                                instruction.immediate64() as i64,
                                false,
                            ));
                        } else {
                            rev_imm_lookup.push(instruction_idx);
                            imm_vec.push(Immediate::Immediate(instruction.immediate(op_idx as u32)));
                        }
                        imms.push((instruction_idx as u32, idx as u32));
                    }
                    OpKind::Memory => {
                        match instruction.mnemonic() {
                            Mnemonic::Nop => {}
                            _ => {
                                let off = instruction.memory_displacement64() as i64 - if instruction.is_ip_rel_memory_operand() {instruction.next_ip() as i64} else {0};
                                if let Some(reloc) = text_section.relocs.get(&(instruction.ip() + mem_off - 0x40_0000)) {
                                    rev_imm_lookup.push(instruction_idx);
                                    imm_vec.push(reloc_to_immediate(
                                        reloc,
                                        &symbols,
                                        &branch_target_table,
                                        &sections,
                                        off,
                                        false,
                                    ));
                                } else {
                                    imm_vec.push(Immediate::Immediate(off as u64));
                                }
                                mem_offsets.push((instruction_idx as u32, idx as u32));
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            for (instruction_idx, idx) in &near_branches {
                text_section.instructions[*instruction_idx as usize].set_near_branch64(*idx as u64);
            }
            for (instruction_idx, idx) in &imms {
                let instruction = &mut text_section.instructions[*instruction_idx as usize];
                for (op_idx, op_kind) in instruction.op_kinds().enumerate() {
                    match op_kind {
                        OpKind::Immediate8 |
                        OpKind::Immediate16 |
                        OpKind::Immediate32 |
                        OpKind::Immediate64 |
                        OpKind::Immediate8to16 |
                        OpKind::Immediate8to32 |
                        OpKind::Immediate8to64 |
                        OpKind::Immediate32to64 => {
                            instruction.set_immediate_u64(op_idx as u32, *idx as u64);
                        }
                        _ => {}
                    }
                }
            }
            for (instruction_idx, idx) in &mem_offsets {
                text_section.instructions[*instruction_idx as usize].set_memory_displacement64(*idx as u64);
            }
            near_branches.clear();
            imms.clear();
            mem_offsets.clear();
        }
        
        let mut data_relocs: Vec<DataReloc> = Vec::new();
        
        let data_section_data = if let Some(data_section) = data_section {
            let data_section_data = &obj[data_section.range];
            for (off, reloc) in data_section.relocs {
                data_relocs.push(DataReloc {
                    size: match reloc.size() {
                        8     => DataRelocSize::I8,
                        16    => DataRelocSize::I16,
                        32    => DataRelocSize::I32,
                        64    => DataRelocSize::I64,
                        other => panic!("Strange data reloc size: {other}"),
                    },
                    imm_idx: imm_vec.len(),
                    off,
                });
                let off = off as usize;
                imm_vec.push(reloc_to_immediate(
                    &reloc,
                    &symbols,
                    &branch_target_table,
                    &sections,
                    match reloc.size() {
                        8  =>  i8::from_le_bytes(data_section_data[off..off+1].try_into().unwrap()) as i64,
                        16 => i16::from_le_bytes(data_section_data[off..off+2].try_into().unwrap()) as i64,
                        32 => i32::from_le_bytes(data_section_data[off..off+4].try_into().unwrap()) as i64,
                        64 => i64::from_le_bytes(data_section_data[off..off+8].try_into().unwrap()),
                        other => panic!("Unexpected relocation size: {other}")
                    },
                    false,
                ));
            }
            Some(data_section_data.to_vec())
        } else {
            None
        };
        
        let mut sym_names: SmallVec<[(String, u64); 64]> = SmallVec::with_capacity(symbols.len()); //there are so few symbols that searching an array of strings is faster than a HashMap
        
        for sym in symbols.values() {
            if let Ok(name) = sym.name() {
                if !name.is_empty() {
                    sym_names.push((name.to_owned(), sym.address()));
                }
            }
        }
        
        sym_names.sort_unstable_by_key(|(_name, addr)| *addr);
        
        let mut patch_syms: HashMap<String, PatchSymbolLocation> = HashMap::with_capacity(256);
        
        for sym in symbols.values() {
            if let Ok(name) = sym.name() {
                if !name.starts_with('_') && !name.starts_with("END") && !name.is_empty() {
                    if let Some(idx) = sym.section_index() {
                        let section = &sections[idx.0 - 1];
                        if let Ok(section_name) = section.name() {
                            patch_syms.insert(name.to_owned(),
                                match section_name {
                                    ".text" => PatchSymbolLocation::Text(*branch_target_table.get(&(sym.address() + 0x40_0000)).unwrap()),
                                    ".data" => PatchSymbolLocation::Data(sym.address()),
                                    _ => {continue;}
                                }
                            );
                        }
                    }
                }
            }
        }
        
        patch_syms.shrink_to_fit();
        
        let mut injections: SmallVec<[Injection; 16]> = SmallVec::new();
        for (original_name, start_off) in sym_names.iter().rev() {
            let search = "END".to_owned() + &original_name;
            for (name, end_off) in &sym_names {
                if name == &search {
                    let start_idx      = *branch_target_table.get(&(start_off + 0x40_0000)).expect("Injection start not instruction aligned");
                    let end_idx        = *branch_target_table.get(&(end_off   + 0x40_0000)).expect("Injection end not instruction aligned");
                    let injection_size = end_idx - start_idx;
                    for (imm_idx, imm) in imm_vec.iter_mut().enumerate() {
                        let mut imm_replace = false;
                        match imm {
                            Immediate::InstructionOffsetCall(off) |
                            Immediate::InstructionOffset(off) => {
                                let orig_off = *off;
                                if *off > start_idx {
                                    *off -= injection_size;
                                }
                                if orig_off == end_idx {
                                    if let Some(instruction_off) = rev_imm_lookup.get(imm_idx) {
                                        if (start_idx..end_idx).contains(instruction_off) {
                                            imm_replace = match imm {
                                                Immediate::InstructionOffset(_) => true,
                                                _ => false,
                                            }
                                        }
                                    }
                                } else if (start_idx..end_idx).contains(&orig_off) {
                                    imm_replace = true;
                                }
                            }
                            _ => {}
                        }
                        if imm_replace {
                            *imm = Immediate::PatchInstructionOffset(injections.len(), *rev_imm_lookup.get(imm_idx).unwrap() - start_idx);
                        }
                    }
                    
                    let mut syms_to_remove: Vec<String> = Vec::new();
                    for (sym_name, sym_loc) in patch_syms.iter_mut() {
                        if let PatchSymbolLocation::Text(off) = sym_loc {
                            let orig_off = *off;
                            if *off > start_idx {
                                *off -= injection_size;
                            }
                            if (start_idx..end_idx).contains(&orig_off) {
                                syms_to_remove.push(sym_name.clone());
                            }
                        }
                    }
                    for sym_name in syms_to_remove {
                        patch_syms.remove(&sym_name);
                    }
                    patch_syms.shrink_to_fit();
                    
                    let mut label_components = original_name.split('+').peekable();
                    let func_name = label_components.next().unwrap().to_owned();
                    let off = u64::from_str_radix(label_components.next().unwrap_or("0x0").strip_prefix("0x").expect("Injection name has offset without 0x prefix"), 16)?;
                    injections.push(Injection {
                        func_name,
                        off,
                        instructions: text_section.instructions.drain(start_idx..end_idx).collect()
                    });
                    break;
                }
            }
        }
        
        Ok(Self {
            injections,
            instructions: text_section.instructions,
            imm_vec,
            data: data_section_data,
            data_relocs,
            patch_syms,
        })
    }
    
    pub fn apply_patches(patches: &[Patch], meta: &IL2CppDumper, fusion: &mut FusionProcess) -> HashMap<String, u64> {
        let mut patches: Vec<Patch> = patches.into_iter().map(|patch| patch.clone()).collect();
        
        let mut il2cpp_syms: HashMap<String, u64> = HashMap::with_capacity(meta.methods_array.len()*3);
        
        for method in &meta.methods_array {
            if method.addr == 0 {
                il2cpp_syms.insert(method.name(meta), 0);
            } else {
                il2cpp_syms.insert(method.name(meta), method.addr - 0x1_8000_0000 + fusion.dll_offset);
                method.get_calls(meta, fusion.dll_offset as i64 - 0x1_8000_0000, &mut il2cpp_syms);
            }
        }
        
        meta.get_field_offsets(&mut il2cpp_syms);
        meta.get_enum_variants(&mut il2cpp_syms);
        
        il2cpp_syms.shrink_to_fit();
        
        //println!("{il2cpp_syms:#?}");
        
        let mut section_offs: Vec<(u64, u64)> = Vec::with_capacity(patches.len());
        let mut data_section_off = fusion.asm_offset;
        for patch in patches.iter_mut() {
            section_offs.push((data_section_off, 0));
            for imm in patch.imm_vec.iter_mut() {
                match imm {
                    Immediate::UnresolvedSymbol(name, addend) => {
                        if let Some(addr) = il2cpp_syms.get(name) {
                            *imm = Immediate::Immediate((*addr as i64 + *addend) as u64)
                        }
                    }
                    Immediate::DataSymbol(off) => {
                        *imm = Immediate::Immediate((data_section_off as i64 + *off) as u64);
                    }
                    _ => {}
                }
            }
            for instruction in &patch.instructions {
                let imm_idxs = get_instruction_imm_idxs(instruction);
                for (imm_kind, imm_idx) in &imm_idxs {
                    let imm = &mut patch.imm_vec[*imm_idx];
                    if let Immediate::DataSymbolRel(off_addr) = imm {
                        let (imm_off, mem_off) = get_instruction_imm_and_memory_offsets(instruction);
                        let off = match imm_kind {
                            OpKind::Memory => {
                                *off_addr + (instruction.len() as i64 - mem_off as i64) 
                            }
                            _ => {
                                *off_addr + (instruction.len() as i64 - imm_off as i64)
                            }
                        };
                        *imm = Immediate::Immediate((data_section_off as i64 + off) as u64)
                    }
                }
            }
            for injection in &patch.injections {
                for instruction in &injection.instructions {
                    let imm_idxs = get_instruction_imm_idxs(instruction);
                    for (imm_kind, imm_idx) in &imm_idxs {
                        let imm = &mut patch.imm_vec[*imm_idx];
                        if let Immediate::DataSymbolRel(off_addr) = imm {
                            let (imm_off, mem_off) = get_instruction_imm_and_memory_offsets(instruction);
                            let off = match imm_kind {
                                OpKind::Memory => {
                                    *off_addr + (instruction.len() as i64 - mem_off as i64) 
                                }
                                _ => {
                                    *off_addr + (instruction.len() as i64 - imm_off as i64)
                                }
                            };
                            *imm = Immediate::Immediate((data_section_off as i64 + off) as u64)
                        }
                    }
                }
            }
            if let Some(data) = &patch.data {
                data_section_off += (data.len() as u64 + 0xF) & !0xF;
            }
        }
        
        let data_section_size = (data_section_off + 0xFFF) & !0xFFF - fusion.asm_offset;
        let mut text_section_off = (data_section_off + 0xFFF) & !0xFFF;
        
        let mut text_off_vecs: Vec<Vec<u32>> = Vec::with_capacity(patches.len());
        
        for (patch_idx, patch) in patches.iter_mut().enumerate() {
            let mut size = ImmSize::Variable;
            for instruction in patch.instructions.iter_mut() {
                match size {
                    ImmSize::Variable |
                    ImmSize::Dword |
                    ImmSize::Largest => {
                        if instruction.is_jcc_short() || instruction.is_jmp_short() {
                            instruction.as_near_branch();
                        }
                        match size {
                            ImmSize::Dword => change_imm_size_dword(instruction),
                            ImmSize::Largest => change_imm_size_large(instruction),
                            _ => change_imm_size_variable(instruction, &patch.imm_vec),
                        }
                    }
                    ImmSize::Smallest => {
                        if instruction.is_jcc_near() || instruction.is_jmp_near() {
                            instruction.as_short_branch();
                        }
                        change_imm_size_small(instruction)
                    }
                }
                match instruction.mnemonic() {
                    Mnemonic::Hlt => {
                        size = ImmSize::Largest;
                    }
                    Mnemonic::Insb => {
                        size = ImmSize::Smallest;
                    }
                    Mnemonic::Insd => {
                        size = ImmSize::Dword;
                    }
                    _ => {
                        size = ImmSize::Variable;
                    }
                }
            }
            
            let mut instruction_offsets_1: Vec<u32> = Vec::with_capacity(patch.instructions.len() + 1);
            let mut encoder = Encoder::new(64);
            
            let mut current_offset = 0;
            for original_instruction in patch.instructions.iter_mut() {
                let instruction_len = match original_instruction.mnemonic() {
                    Mnemonic::Hlt |
                    Mnemonic::Insb |
                    Mnemonic::Insd => 0,
                    _ => {
                        let mut instruction = original_instruction.clone();
                        fill_in_imms_phase_1(&mut instruction, &patch.imm_vec, text_section_off + current_offset as u64, fusion.dll_offset);
                        match encoder.encode(&instruction, text_section_off + current_offset as u64) {
                            Ok(sz) => sz as u32,
                            Err(err) => {
                                let mut instr_string = String::new();
                                let mut formatter = GasFormatter::with_options(None, None);
                                formatter.format(&instruction, &mut instr_string);
                                panic!("Phase 1 encoding error: {err}\n{}", instr_string);
                            }
                        }
                    }
                };
                instruction_offsets_1.push(current_offset);
                current_offset += instruction_len;
            }
            instruction_offsets_1.push(current_offset);
            
            for (i, instruction) in patch.instructions.iter_mut().enumerate() {
                instruction.set_next_ip(instruction_offsets_1[i+1] as u64 + text_section_off);
                instruction.set_len((instruction_offsets_1[i+1] - instruction_offsets_1[i]) as usize);
            }
            
            let mut size = ImmSize::Variable;
            for (i, instruction) in patch.instructions.iter_mut().enumerate() {
                if let ImmSize::Variable = size {
                    change_imm_size_variable_v2(instruction, i, &patch.imm_vec, &instruction_offsets_1)
                }
                match instruction.mnemonic() {
                    Mnemonic::Hlt => {
                        size = ImmSize::Largest;
                    }
                    Mnemonic::Insb => {
                        size = ImmSize::Smallest;
                    }
                    Mnemonic::Insd => {
                        size = ImmSize::Dword;
                    }
                    _ => {
                        size = ImmSize::Variable;
                    }
                }
            }
            
            let mut instruction_offsets_2: Vec<u32> = Vec::with_capacity(patch.instructions.len() + 1);
            let mut encoder = Encoder::new(64);
            
            let mut current_offset = 0;
            for original_instruction in patch.instructions.iter() {
                let instruction_len = match original_instruction.mnemonic() {
                    Mnemonic::Hlt |
                    Mnemonic::Insb |
                    Mnemonic::Insd => 0,
                    _ => {
                        let mut instruction = original_instruction.clone();
                        fill_in_imms_phase_1(&mut instruction, &patch.imm_vec, text_section_off + current_offset as u64, fusion.dll_offset);
                        encoder.encode(&instruction, text_section_off + current_offset as u64).expect("Phase 2 encoding error") as u32
                    }
                };
                instruction_offsets_2.push(current_offset);
                current_offset += instruction_len;
            }
            instruction_offsets_2.push(current_offset);
            
            for (i, instruction) in patch.instructions.iter_mut().enumerate() {
                instruction.set_next_ip(instruction_offsets_2[i+1] as u64 + text_section_off);
                instruction.set_len((instruction_offsets_2[i+1] - instruction_offsets_2[i]) as usize);
            }
            
            section_offs[patch_idx].1 = text_section_off;
            text_off_vecs.push(instruction_offsets_2);
            text_section_off += current_offset as u64;
        }
        
        let text_section_size = ((text_section_off + 0xFFF) & !0xFFF) - data_section_size - fusion.asm_offset;
        
        //fusion.allocate_memory(fusion.asm_offset, data_section_size, PAGE_READWRITE);
        //fusion.allocate_memory(fusion.asm_offset + data_section_size, text_section_size, PAGE_EXECUTE_READ);
        
        //I was having some difficulty with the second mapping, so I'm beind lazy and doing one RWX mapping
        fusion.allocate_memory(fusion.asm_offset, data_section_size + text_section_size, PAGE_EXECUTE_READWRITE);
        
        //let mut sym_tab: HashMap<String,u64> = HashMap::new();
        
        for (patch_idx, patch) in patches.iter().enumerate() {
            for (sym_name, sym_location) in &patch.patch_syms {
                match sym_location {
                    PatchSymbolLocation::Text(instruction_idx) => {
                        il2cpp_syms.insert(sym_name.clone(), section_offs[patch_idx].1 + text_off_vecs[patch_idx][*instruction_idx] as u64);
                    }
                    PatchSymbolLocation::Data(off) => {
                        il2cpp_syms.insert(sym_name.clone(), section_offs[patch_idx].0 + off);
                    }
                }
            }
        }
        
        let mut encoder = Encoder::new(64);
        
        for ((patch, (_data_section_off, text_section_off)), instruction_offsets) in patches.iter().zip(section_offs.iter()).zip(text_off_vecs.iter()) {
            let mut current_offset = 0;
            for original_instruction in patch.instructions.iter() {
                let instruction_len = match original_instruction.mnemonic() {
                    Mnemonic::Hlt |
                    Mnemonic::Insb |
                    Mnemonic::Insd => 0,
                    _ => {
                        let mut instruction = original_instruction.clone();
                        fill_in_imms_phase_2(
                            &mut instruction,
                            &patch.imm_vec,
                            instruction_offsets,
                            *text_section_off,
                            &il2cpp_syms,
                            None,
                            None,
                            None,
                        );
                        encoder.encode(&instruction, text_section_off + current_offset as u64).expect("Phase 2 encoding error") as u32
                    }
                };
                current_offset += instruction_len;
            }
        }
        
        let code = encoder.take_buffer();
        
        fusion.write_memory(fusion.asm_offset + data_section_size, &code).unwrap();
        
        let mut all_data: Vec<u8> = Vec::with_capacity(text_section_size as usize);
        
        for (patch_idx, patch) in patches.iter_mut().enumerate() {
            if let Some(data) = &mut patch.data {
                for data_reloc in &patch.data_relocs {
                    let imm = &patch.imm_vec[data_reloc.imm_idx];
                    let patch_val = match imm {
                        Immediate::Immediate(imm) => *imm,
                        Immediate::PatchInstructionOffset(_, _) => panic!("Patch instruction offset found in data section"),
                        Immediate::InstructionOffset(idx) |
                        Immediate::InstructionOffsetCall(idx) => text_off_vecs[patch_idx][*idx] as u64 + section_offs[patch_idx].1,
                        Immediate::UnresolvedSymbol(name, addend) =>
                            match il2cpp_syms.get(name) {
                                None => panic!("Could not find unresolved symbol {name}"),
                                Some(x) => (*x as i64 + *addend) as u64,
                            }
                        _ => panic!("Weird data relocation type")
                    };
                    match data_reloc.size {
                        DataRelocSize::I8 => data[data_reloc.off as usize] = patch_val as u8,
                        DataRelocSize::I16 => {
                            for (i, byte) in (patch_val as u16).to_le_bytes().iter().enumerate() {
                                data[data_reloc.off as usize + i] = *byte;
                            }
                        }
                        DataRelocSize::I32 => {
                            for (i, byte) in (patch_val as u32).to_le_bytes().iter().enumerate() {
                                data[data_reloc.off as usize + i] = *byte;
                            }
                        }
                        DataRelocSize::I64 => {
                            for (i, byte) in (patch_val as u64).to_le_bytes().iter().enumerate() {
                                data[data_reloc.off as usize + i] = *byte;
                            }
                        }
                    }
                }
                let pad_len = ((all_data.len() + 0xF) & !0xF) - all_data.len();
                all_data.write(&vec![0; pad_len]).unwrap();
                all_data.write(data).unwrap();
            }
        }
        
        if false {
            println!("DATA:");
            for byte in &all_data {
                print!("{:02X} ", byte);
            }
            println!("")
        }
        
        fusion.write_memory(fusion.asm_offset, &all_data).unwrap();
        
        for (patch_idx, patch) in patches.iter_mut().enumerate() {
            let (_data_section_off, text_section_off) = &section_offs[patch_idx];
            for injection in patch.injections.iter_mut() {
                injection.off += il2cpp_syms.get(&injection.func_name).unwrap();
                let method = &meta.methods_array[*meta.methods_table.get(&injection.func_name).expect("Invalid injection function name") as usize];
                let local_syms = method.get_local_syms(fusion.dll_offset as i64 - 0x1_8000_0000, meta);
                
                let mut size = ImmSize::Variable;
                for instruction in injection.instructions.iter_mut() {
                    match size {
                        ImmSize::Variable |
                        ImmSize::Dword |
                        ImmSize::Largest => {
                            if instruction.is_jcc_short() || instruction.is_jmp_short() {
                                instruction.as_near_branch();
                            }
                            match size {
                                ImmSize::Dword => change_imm_size_dword(instruction),
                                ImmSize::Largest => change_imm_size_large(instruction),
                                _ => change_imm_size_variable(instruction, &patch.imm_vec),
                            }
                        }
                        ImmSize::Smallest => {
                            if instruction.is_jcc_near() || instruction.is_jmp_near() {
                                instruction.as_short_branch();
                            }
                            change_imm_size_small(instruction)
                        }
                    }
                    match instruction.mnemonic() {
                        Mnemonic::Hlt => {
                            size = ImmSize::Largest;
                        }
                        Mnemonic::Insb => {
                            size = ImmSize::Smallest;
                        }
                        Mnemonic::Insd => {
                            size = ImmSize::Dword;
                        }
                        _ => {
                            size = ImmSize::Variable;
                        }
                    }
                }
                
                let mut instruction_offsets_1: Vec<u32> = Vec::with_capacity(injection.instructions.len() + 1);
                let mut encoder = Encoder::new(64);
                
                let mut current_offset = 0;
                for original_instruction in injection.instructions.iter_mut() {
                    let instruction_len = match original_instruction.mnemonic() {
                        Mnemonic::Hlt |
                        Mnemonic::Insb |
                        Mnemonic::Insd => 0,
                        _ => {
                            let mut instruction = original_instruction.clone();
                            fill_in_imms_phase_1(&mut instruction, &patch.imm_vec, injection.off + current_offset as u64, fusion.dll_offset);
                            encoder.encode(&instruction, injection.off + current_offset as u64).expect("Phase 1 encoding error") as u32
                        }
                    };
                    instruction_offsets_1.push(current_offset);
                    current_offset += instruction_len;
                }
                instruction_offsets_1.push(current_offset);
                
                for (i, instruction) in injection.instructions.iter_mut().enumerate() {
                    instruction.set_next_ip(instruction_offsets_1[i+1] as u64 + injection.off);
                    instruction.set_len((instruction_offsets_1[i+1] - instruction_offsets_1[i]) as usize);
                }
                
                let mut size = ImmSize::Variable;
                for (i, instruction) in injection.instructions.iter_mut().enumerate() {
                    if let ImmSize::Variable = size {
                        change_imm_size_variable_v3(instruction, i, &patch.imm_vec, &instruction_offsets_1)
                    }
                    match instruction.mnemonic() {
                        Mnemonic::Hlt => {
                            size = ImmSize::Largest;
                        }
                        Mnemonic::Insb => {
                            size = ImmSize::Smallest;
                        }
                        Mnemonic::Insd => {
                            size = ImmSize::Dword;
                        }
                        _ => {
                            size = ImmSize::Variable;
                        }
                    }
                }
                
                let mut instruction_offsets_2: Vec<u32> = Vec::with_capacity(injection.instructions.len() + 1);
                let mut encoder = Encoder::new(64);
                
                let mut current_offset = 0;
                for original_instruction in injection.instructions.iter() {
                    let instruction_len = match original_instruction.mnemonic() {
                        Mnemonic::Hlt |
                        Mnemonic::Insb |
                        Mnemonic::Insd => 0,
                        _ => {
                            let mut instruction = original_instruction.clone();
                            fill_in_imms_phase_1(&mut instruction, &patch.imm_vec, injection.off + current_offset as u64, fusion.dll_offset);
                            encoder.encode(&instruction, injection.off + current_offset as u64).expect("Phase 2 encoding error") as u32
                        }
                    };
                    instruction_offsets_2.push(current_offset);
                    current_offset += instruction_len;
                }
                instruction_offsets_2.push(current_offset);
                
                for (i, instruction) in injection.instructions.iter_mut().enumerate() {
                    instruction.set_next_ip(instruction_offsets_2[i+1] as u64 + injection.off);
                    instruction.set_len((instruction_offsets_2[i+1] - instruction_offsets_2[i]) as usize);
                }
                let mut encoder = Encoder::new(64);
                
                let mut current_offset = 0;
                for original_instruction in injection.instructions.iter() {
                    let instruction_len = match original_instruction.mnemonic() {
                        Mnemonic::Hlt |
                        Mnemonic::Insb |
                        Mnemonic::Insd => 0,
                        _ => {
                            let mut instruction = original_instruction.clone();
                            fill_in_imms_phase_2(
                                &mut instruction,
                                &patch.imm_vec,
                                &text_off_vecs[patch_idx],
                                *text_section_off,
                                &il2cpp_syms,
                                Some(&instruction_offsets_2),
                                Some(NonZeroU64::new(injection.off).unwrap()),
                                Some(&local_syms),
                            );
                            encoder.encode(&instruction, injection.off + current_offset as u64).expect("Phase 2 encoding error") as u32
                        }
                    };
                    current_offset += instruction_len;
                }
                
                let code = encoder.take_buffer();
                fusion.write_memory(injection.off, &code).unwrap();
            }
        }
        
        il2cpp_syms
    }
}

#[allow(dead_code)]
#[derive(Debug)]
#[derive(Clone)]
pub struct Injection {
    func_name:    String,
    off:          u64,
    instructions: Vec<Instruction>,
}

#[allow(dead_code)]
#[derive(Debug)]
#[derive(Clone)]
pub struct DataReloc {
    size: DataRelocSize,
    imm_idx: usize,
    off: u64,
} 

#[allow(dead_code)]
#[derive(Debug)]
#[derive(Clone)]
enum Immediate {
    PatchInstructionOffset(usize, usize),
    InstructionOffset(usize),
    InstructionOffsetCall(usize),
    UnresolvedSymbol(String, i64),
    UnresolvedSymbolRel(String, i64),
    DataSymbol(i64),
    DataSymbolRel(i64),
    Immediate(u64),
}

#[allow(dead_code)]
#[derive(Debug)]
#[derive(Clone)]
enum PatchSymbolLocation {
    Text(usize),
    Data(u64),
}

enum ImmSize {
    Variable, //no prefix
    Smallest, //insb
    Dword,    //insd, only for mov immediate
    Largest,  //hlt
}

#[allow(dead_code)]
#[derive(Debug)]
#[derive(Clone)]
enum DataRelocSize {
    I8,
    I16,
    I32,
    I64,
}

fn get_instruction_imm_idxs(instruction: &Instruction) -> SmallVec<[(OpKind,usize);2]> {
    let mut ret = SmallVec::new();
    for (i, op) in instruction.op_kinds().enumerate() {
        match op {
            OpKind::NearBranch64 => {
                ret.push((op, instruction.near_branch_target() as usize));
            }
            OpKind::Memory => {
                match instruction.mnemonic() {
                    Mnemonic::Nop => {}
                    _ => {
                        ret.push((op, instruction.memory_displacement64() as usize));
                    }
                }
            }
            OpKind::Immediate8 |
            OpKind::Immediate16 |
            OpKind::Immediate32 |
            OpKind::Immediate64 |
            OpKind::Immediate8to16 |
            OpKind::Immediate8to32 |
            OpKind::Immediate8to64 |
            OpKind::Immediate32to64 => {
                ret.push((op, instruction.immediate(i as u32) as usize))
            }
            _ => {}
        }
    }
    ret
}

fn fill_in_imms_phase_2(
    instruction: &mut Instruction,
    imm_vec: &[Immediate],
    offsets: &[u32],
    text_section_off: u64,
    sym_tab: &HashMap<String,u64>,
    patch_offs: Option<&[u32]>,
    patch_addr: Option<NonZeroU64>,
    local_sym_tab: Option<&HashMap<String,u64>>,
) {
    for (i, op) in instruction.op_kinds().enumerate() {
        match op {
            OpKind::NearBranch64 => {
                let imm = &imm_vec[instruction.near_branch_target() as usize];
                match imm {
                    Immediate::Immediate(value) => {
                        instruction.set_near_branch64(*value);
                    }
                    Immediate::UnresolvedSymbol(sym_name, addend) |
                    Immediate::UnresolvedSymbolRel(sym_name, addend) => {
                        if let Some(addr) = sym_tab.get(sym_name) {
                            instruction.set_near_branch64((*addr as i64 + 4 + addend) as u64); //this +4 is important TODO: only add 4 for relative
                        } else if let Some(local_sym_tab) = local_sym_tab {
                            let addr = local_sym_tab.get(&format!("\"{sym_name}\"")).expect("Undefined symbol found while filling in immediates (phase 2)");
                            instruction.set_near_branch64((*addr as i64 + 4 + addend) as u64); //this +4 is important TODO: only add 4 for relative
                        } else {
                            panic!("Undefined symbol found while filling in immediates (phase 2)");
                        }
                    }
                    Immediate::InstructionOffset(idx) |
                    Immediate::InstructionOffsetCall(idx) => {
                        instruction.set_near_branch64(offsets[*idx] as u64 + text_section_off);
                    }
                    Immediate::PatchInstructionOffset(_, idx) => {
                        if let Some(patch_offs) = patch_offs {
                            let patch_addr = patch_addr.unwrap();
                            instruction.set_near_branch64(patch_offs[*idx] as u64 + patch_addr.get());
                        }
                    }
                    _ => {}
                }
            }
            OpKind::Memory => {
                match instruction.mnemonic() {
                    Mnemonic::Nop => {}
                    _ => {
                        let imm = &imm_vec[instruction.memory_displacement64() as usize];
                        match imm {
                            Immediate::Immediate(value) => {
                                instruction.set_memory_displacement64(*value);
                            }
                            Immediate::UnresolvedSymbol(sym_name, addend) |
                            Immediate::UnresolvedSymbolRel(sym_name, addend) => {
                                if instruction.is_ip_rel_memory_operand() {
                                    let addr = sym_tab.get(sym_name).expect("Undefined symbol found while filling in immediates (phase 2)");
                                    let (_, mem_off) = get_instruction_imm_and_memory_offsets_2(instruction);
                                    instruction.set_memory_displacement64((*addr as i64 + mem_off as i64 + addend) as u64); //hopefully instruction hasn't been shortened...
                                } else {
                                    let mut instr_string = String::new();
                                    let mut formatter = GasFormatter::with_options(None, None);
                                    formatter.format(instruction, &mut instr_string);
                                    panic!("Unresolved memory access is not ip relative!\n{instr_string}");
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            OpKind::Immediate8 |
            OpKind::Immediate16 |
            OpKind::Immediate32 |
            OpKind::Immediate64 |
            OpKind::Immediate8to16 |
            OpKind::Immediate8to32 |
            OpKind::Immediate8to64 |
            OpKind::Immediate32to64 => {
                let imm = &imm_vec[instruction.immediate(i as u32) as usize];
                match imm {
                    Immediate::Immediate(value) => {
                        instruction.set_immediate_u64(i as u32, *value);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn fill_in_imms_phase_1(instruction: &mut Instruction, imm_vec: &[Immediate], ip: u64, dll_base: u64) {
    for (i, op) in instruction.op_kinds().enumerate() {
        match op {
            OpKind::NearBranch64 => {
                let imm = &imm_vec[instruction.near_branch_target() as usize];
                match imm {
                    Immediate::Immediate(value) => {
                        instruction.set_near_branch64(*value);
                    }
                    Immediate::UnresolvedSymbol(_, _) |
                    Immediate::UnresolvedSymbolRel(_, _) => {
                        instruction.set_near_branch64(ip);
                    }
                    Immediate::InstructionOffset(_idx) |
                    Immediate::InstructionOffsetCall(_idx) => {
                        instruction.set_near_branch64(ip);
                    } 
                    Immediate::PatchInstructionOffset(_patch_idx, _instruction_idx) => {
                        instruction.set_near_branch64(ip);
                    },
                    _ => {}
                }
            }
            OpKind::Memory => {
                match instruction.mnemonic() {
                    Mnemonic::Nop => {}
                    _ => {
                        let imm = &imm_vec[instruction.memory_displacement64() as usize];
                        match imm {
                            Immediate::Immediate(value) => {
                                instruction.set_memory_displacement64(*value);
                            }
                            Immediate::UnresolvedSymbol(_, _) |
                            Immediate::UnresolvedSymbolRel(_, _) => {
                                if instruction.is_ip_rel_memory_operand() {
                                    instruction.set_memory_displacement64(dll_base);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            OpKind::Immediate8 |
            OpKind::Immediate16 |
            OpKind::Immediate32 |
            OpKind::Immediate64 |
            OpKind::Immediate8to16 |
            OpKind::Immediate8to32 |
            OpKind::Immediate8to64 |
            OpKind::Immediate32to64 => {
                let imm = &imm_vec[instruction.immediate(i as u32) as usize];
                match imm {
                    Immediate::Immediate(value) => {
                        instruction.set_immediate_u64(i as u32, *value);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn change_imm_size_variable_v3(instruction: &mut Instruction, instruction_idx: usize, imm_vec: &[Immediate], offsets: &[u32]) {
    change_imm_size_variable(instruction, imm_vec);
    for op in instruction.op_kinds() {
        match op {
            OpKind::NearBranch64 => {
                if let Immediate::PatchInstructionOffset(_, idx) = imm_vec[instruction.near_branch64() as usize] {
                    let branch_val = offsets[idx] as i64 - offsets[instruction_idx+1] as i64;
                    if i8::try_from(branch_val).is_ok() {
                        instruction.as_short_branch();
                    } else {
                        instruction.as_near_branch();
                    }
                }
            }
            _ => {}
        }
    }
}

fn change_imm_size_variable_v2(instruction: &mut Instruction, instruction_idx: usize, imm_vec: &[Immediate], offsets: &[u32]) {
    change_imm_size_variable(instruction, imm_vec);
    for op in instruction.op_kinds() {
        match op {
            OpKind::NearBranch64 => {
                if let Immediate::InstructionOffset(idx) = imm_vec[instruction.near_branch64() as usize] {
                    let branch_val = offsets[idx] as i64 - offsets[instruction_idx+1] as i64;
                    if i8::try_from(branch_val).is_ok() {
                        instruction.as_short_branch();
                    } else {
                        instruction.as_near_branch();
                    }
                }
            }
            _ => {}
        }
    }
}

fn change_imm_size_variable(instruction: &mut Instruction, imm_vec: &[Immediate]) {
    for (i, op) in instruction.op_kinds().enumerate() {
        match op {
            OpKind::Memory => {
                if let Mnemonic::Nop = instruction.mnemonic() {
                } else if instruction.memory_displ_size() != 0 {
                    if instruction.is_ip_rel_memory_operand() {
                        instruction.set_memory_displ_size(8);
                    } else {
                        match imm_vec[instruction.memory_displacement64() as usize] {
                            Immediate::Immediate(off) => {
                                if i8::try_from(off as i64).is_ok() {
                                    instruction.set_memory_displ_size(1);
                                } else {
                                    instruction.set_memory_displ_size(8);
                                }
                            }
                            _ => instruction.set_memory_displ_size(8)
                        }
                    }
                }
            }
            OpKind::Immediate8 |
            OpKind::Immediate16 |
            OpKind::Immediate32 |
            OpKind::Immediate64 |
            OpKind::Immediate8to16 |
            OpKind::Immediate8to32 |
            OpKind::Immediate8to64 |
            OpKind::Immediate32to64 => {
                let immediate = &imm_vec[instruction.immediate(i as u32) as usize];
                match immediate {
                    Immediate::Immediate(value) => {
                        let fits_into_i8  =  i8::try_from(*value as i64).is_ok();
                        let fits_into_i32 = i32::try_from(*value as i64).is_ok();
                        let mut opcode = instruction.code();
                        let mut imm;
                        opcode = if fits_into_i8 {
                            imm = match instruction.op1_kind() {
                                OpKind::Immediate16 => OpKind::Immediate8to16,
                                OpKind::Immediate32 => OpKind::Immediate8to32,
                                OpKind::Immediate32to64 |
                                OpKind::Immediate64 => OpKind::Immediate8to64,
                                _ => OpKind::Register
                            };
                            match opcode {
                                Code::Adc_rm16_imm16 => Code::Adc_rm16_imm8,
                                Code::Adc_rm32_imm32 => Code::Adc_rm32_imm8,
                                Code::Adc_rm64_imm32 => Code::Adc_rm64_imm8,
                                Code::Add_rm16_imm16 => Code::Add_rm16_imm8,
                                Code::Add_rm32_imm32 => Code::Add_rm32_imm8,
                                Code::Add_rm64_imm32 => Code::Add_rm64_imm8,
                                Code::And_rm16_imm16 => Code::And_rm16_imm8,
                                Code::And_rm32_imm32 => Code::And_rm32_imm8,
                                Code::And_rm64_imm32 => Code::And_rm64_imm8,
                                Code::Cmp_rm16_imm16 => Code::Cmp_rm16_imm8,
                                Code::Cmp_rm32_imm32 => Code::Cmp_rm32_imm8,
                                Code::Cmp_rm64_imm32 => Code::Cmp_rm64_imm8,
                                Code::Imul_r16_rm16_imm16 => Code::Imul_r16_rm16_imm8,
                                Code::Imul_r32_rm32_imm32 => Code::Imul_r32_rm32_imm8,
                                Code::Imul_r64_rm64_imm32 => Code::Imul_r64_rm64_imm8,
                                Code::Or_rm16_imm16 => Code::Or_rm16_imm8,
                                Code::Or_rm32_imm32 => Code::Or_rm32_imm8,
                                Code::Or_rm64_imm32 => Code::Or_rm64_imm8,
                                Code::Pushq_imm32 => Code::Pushq_imm8,
                                Code::Sbb_rm16_imm16 => Code::Sbb_rm16_imm8,
                                Code::Sbb_rm32_imm32 => Code::Sbb_rm32_imm8,
                                Code::Sbb_rm64_imm32 => Code::Sbb_rm64_imm8,
                                Code::Sub_rm16_imm16 => Code::Sub_rm16_imm8,
                                Code::Sub_rm32_imm32 => Code::Sub_rm32_imm8,
                                Code::Sub_rm64_imm32 => Code::Sub_rm64_imm8,
                                Code::Xor_rm16_imm16 => Code::Xor_rm16_imm8,
                                Code::Xor_rm32_imm32 => Code::Xor_rm32_imm8,
                                Code::Xor_rm64_imm32 => Code::Xor_rm64_imm8,
                                other => {imm = OpKind::Register; other}
                            }
                        } else {
                            imm = match instruction.op1_kind() {
                                OpKind::Immediate8to16 => OpKind::Immediate16,
                                OpKind::Immediate8to32 => OpKind::Immediate32,
                                OpKind::Immediate8to64 => OpKind::Immediate64,
                                _ => OpKind::Register
                            };
                            match opcode {
                                Code::Adc_rm16_imm8 => Code::Adc_rm16_imm16,
                                Code::Adc_rm32_imm8 => Code::Adc_rm32_imm32,
                                Code::Adc_rm64_imm8 => Code::Adc_rm64_imm32,
                                Code::Add_rm16_imm8 => Code::Add_rm16_imm16,
                                Code::Add_rm32_imm8 => Code::Add_rm32_imm32,
                                Code::Add_rm64_imm8 => Code::Add_rm64_imm32,
                                Code::And_rm16_imm8 => Code::And_rm16_imm16,
                                Code::And_rm32_imm8 => Code::And_rm32_imm32,
                                Code::And_rm64_imm8 => Code::And_rm64_imm32,
                                Code::Cmp_rm16_imm8 => Code::Cmp_rm16_imm16,
                                Code::Cmp_rm32_imm8 => Code::Cmp_rm32_imm32,
                                Code::Cmp_rm64_imm8 => Code::Cmp_rm64_imm32,
                                Code::Imul_r16_rm16_imm8 => Code::Imul_r16_rm16_imm16,
                                Code::Imul_r32_rm32_imm8 => Code::Imul_r32_rm32_imm32,
                                Code::Imul_r64_rm64_imm8 => Code::Imul_r64_rm64_imm32,
                                Code::Or_rm16_imm8 => Code::Or_rm16_imm16,
                                Code::Or_rm32_imm8 => Code::Or_rm32_imm32,
                                Code::Or_rm64_imm8 => Code::Or_rm64_imm32,
                                Code::Pushq_imm8 => Code::Pushq_imm32,
                                Code::Sbb_rm16_imm8 => Code::Sbb_rm16_imm16,
                                Code::Sbb_rm32_imm8 => Code::Sbb_rm32_imm32,
                                Code::Sbb_rm64_imm8 => Code::Sbb_rm64_imm32,
                                Code::Sub_rm16_imm8 => Code::Sub_rm16_imm16,
                                Code::Sub_rm32_imm8 => Code::Sub_rm32_imm32,
                                Code::Sub_rm64_imm8 => Code::Sub_rm64_imm32,
                                Code::Xor_rm16_imm8 => Code::Xor_rm16_imm16,
                                Code::Xor_rm32_imm8 => Code::Xor_rm32_imm32,
                                Code::Xor_rm64_imm8 => Code::Xor_rm64_imm32,
                                other => {imm = OpKind::Register; other}
                            }
                        };
                        (opcode, imm) = match opcode {
                            Code::Mov_r64_imm64 |
                            Code::Mov_rm64_imm32 => {
                                if fits_into_i32 {
                                    (Code::Mov_rm64_imm32, OpKind::Immediate32to64)
                                } else {
                                    (Code::Mov_r64_imm64, OpKind::Immediate64)
                                }
                            }
                            other => (other, imm) //ignore value
                        };
                        instruction.set_code(opcode);
                        match imm {
                            OpKind::Register => {}
                            _ => match instruction.mnemonic() {
                                Mnemonic::Imul => instruction.set_op2_kind(imm),
                                Mnemonic::Push => instruction.set_op0_kind(imm),
                                _              => instruction.set_op1_kind(imm),
                            }
                        }
                    }
                    _ => {
                        match instruction.code() {
                            Code::Adc_rm16_imm8 => {instruction.set_code(Code::Adc_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::Adc_rm32_imm8 => {instruction.set_code(Code::Adc_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::Adc_rm64_imm8 => {instruction.set_code(Code::Adc_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            Code::Add_rm16_imm8 => {instruction.set_code(Code::Add_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::Add_rm32_imm8 => {instruction.set_code(Code::Add_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::Add_rm64_imm8 => {instruction.set_code(Code::Add_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            Code::And_rm16_imm8 => {instruction.set_code(Code::And_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::And_rm32_imm8 => {instruction.set_code(Code::And_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::And_rm64_imm8 => {instruction.set_code(Code::And_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            Code::Cmp_rm16_imm8 => {instruction.set_code(Code::Cmp_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::Cmp_rm32_imm8 => {instruction.set_code(Code::Cmp_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::Cmp_rm64_imm8 => {instruction.set_code(Code::Cmp_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            Code::Imul_r16_rm16_imm8 => {instruction.set_code(Code::Imul_r16_rm16_imm16); instruction.set_op2_kind(OpKind::Immediate16)},
                            Code::Imul_r32_rm32_imm8 => {instruction.set_code(Code::Imul_r32_rm32_imm32); instruction.set_op2_kind(OpKind::Immediate32)},
                            Code::Imul_r64_rm64_imm8 => {instruction.set_code(Code::Imul_r64_rm64_imm32); instruction.set_op2_kind(OpKind::Immediate32to64)},
                            //Code::Mov_r32_imm32 => {instruction.set_code(Code::Mov_r64_imm64); instruction.set_op1_kind(OpKind::Immediate64)},
                            Code::Or_rm16_imm8 => {instruction.set_code(Code::Or_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::Or_rm32_imm8 => {instruction.set_code(Code::Or_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::Or_rm64_imm8 => {instruction.set_code(Code::Or_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            Code::Pushq_imm8 => {instruction.set_code(Code::Pushq_imm32); instruction.set_op0_kind(OpKind::Immediate32to64)},
                            Code::Sbb_rm16_imm8 => {instruction.set_code(Code::Sbb_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::Sbb_rm32_imm8 => {instruction.set_code(Code::Sbb_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::Sbb_rm64_imm8 => {instruction.set_code(Code::Sbb_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            Code::Sub_rm16_imm8 => {instruction.set_code(Code::Sub_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::Sub_rm32_imm8 => {instruction.set_code(Code::Sub_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::Sub_rm64_imm8 => {instruction.set_code(Code::Sub_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            Code::Xor_rm16_imm8 => {instruction.set_code(Code::Xor_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
                            Code::Xor_rm32_imm8 => {instruction.set_code(Code::Xor_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
                            Code::Xor_rm64_imm8 => {instruction.set_code(Code::Xor_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
                            _ => {}
                        }
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}

fn change_imm_size_large(instruction: &mut Instruction) {
    match instruction.code() {
        Code::Adc_rm16_imm8 => {instruction.set_code(Code::Adc_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Adc_rm32_imm8 => {instruction.set_code(Code::Adc_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Adc_rm64_imm8 => {instruction.set_code(Code::Adc_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Add_rm16_imm8 => {instruction.set_code(Code::Add_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Add_rm32_imm8 => {instruction.set_code(Code::Add_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Add_rm64_imm8 => {instruction.set_code(Code::Add_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::And_rm16_imm8 => {instruction.set_code(Code::And_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::And_rm32_imm8 => {instruction.set_code(Code::And_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::And_rm64_imm8 => {instruction.set_code(Code::And_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Cmp_rm16_imm8 => {instruction.set_code(Code::Cmp_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Cmp_rm32_imm8 => {instruction.set_code(Code::Cmp_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Cmp_rm64_imm8 => {instruction.set_code(Code::Cmp_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Imul_r16_rm16_imm8 => {instruction.set_code(Code::Imul_r16_rm16_imm16); instruction.set_op2_kind(OpKind::Immediate16)},
        Code::Imul_r32_rm32_imm8 => {instruction.set_code(Code::Imul_r32_rm32_imm32); instruction.set_op2_kind(OpKind::Immediate32)},
        Code::Imul_r64_rm64_imm8 => {instruction.set_code(Code::Imul_r64_rm64_imm32); instruction.set_op2_kind(OpKind::Immediate32to64)},
        //Code::Mov_r32_imm32 => {instruction.set_code(Code::Mov_r64_imm64); instruction.set_op1_kind(OpKind::Immediate64)},
        Code::Or_rm16_imm8 => {instruction.set_code(Code::Or_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Or_rm32_imm8 => {instruction.set_code(Code::Or_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Or_rm64_imm8 => {instruction.set_code(Code::Or_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Pushq_imm8 => {instruction.set_code(Code::Pushq_imm32); instruction.set_op0_kind(OpKind::Immediate32to64)},
        Code::Sbb_rm16_imm8 => {instruction.set_code(Code::Sbb_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Sbb_rm32_imm8 => {instruction.set_code(Code::Sbb_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Sbb_rm64_imm8 => {instruction.set_code(Code::Sbb_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Sub_rm16_imm8 => {instruction.set_code(Code::Sub_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Sub_rm32_imm8 => {instruction.set_code(Code::Sub_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Sub_rm64_imm8 => {instruction.set_code(Code::Sub_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Xor_rm16_imm8 => {instruction.set_code(Code::Xor_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Xor_rm32_imm8 => {instruction.set_code(Code::Xor_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Xor_rm64_imm8 => {instruction.set_code(Code::Xor_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        _ => {}
    }
    for op in instruction.op_kinds() {
        match op {
            OpKind::Memory => {
                if instruction.memory_displ_size() != 0 {
                    instruction.set_memory_displ_size(8);
                }
            }
            _ => {}
        }
    }
}

fn change_imm_size_dword(instruction: &mut Instruction) {
    match instruction.code() {
        Code::Adc_rm16_imm8 => {instruction.set_code(Code::Adc_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Adc_rm32_imm8 => {instruction.set_code(Code::Adc_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Adc_rm64_imm8 => {instruction.set_code(Code::Adc_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Add_rm16_imm8 => {instruction.set_code(Code::Add_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Add_rm32_imm8 => {instruction.set_code(Code::Add_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Add_rm64_imm8 => {instruction.set_code(Code::Add_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::And_rm16_imm8 => {instruction.set_code(Code::And_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::And_rm32_imm8 => {instruction.set_code(Code::And_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::And_rm64_imm8 => {instruction.set_code(Code::And_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Cmp_rm16_imm8 => {instruction.set_code(Code::Cmp_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Cmp_rm32_imm8 => {instruction.set_code(Code::Cmp_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Cmp_rm64_imm8 => {instruction.set_code(Code::Cmp_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Imul_r16_rm16_imm8 => {instruction.set_code(Code::Imul_r16_rm16_imm16); instruction.set_op2_kind(OpKind::Immediate16)},
        Code::Imul_r32_rm32_imm8 => {instruction.set_code(Code::Imul_r32_rm32_imm32); instruction.set_op2_kind(OpKind::Immediate32)},
        Code::Imul_r64_rm64_imm8 => {instruction.set_code(Code::Imul_r64_rm64_imm32); instruction.set_op2_kind(OpKind::Immediate32to64)},
        Code::Mov_r64_imm64 => {instruction.set_code(Code::Mov_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Or_rm16_imm8 => {instruction.set_code(Code::Or_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Or_rm32_imm8 => {instruction.set_code(Code::Or_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Or_rm64_imm8 => {instruction.set_code(Code::Or_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Pushq_imm8 => {instruction.set_code(Code::Pushq_imm32); instruction.set_op0_kind(OpKind::Immediate32to64)},
        Code::Sbb_rm16_imm8 => {instruction.set_code(Code::Sbb_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Sbb_rm32_imm8 => {instruction.set_code(Code::Sbb_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Sbb_rm64_imm8 => {instruction.set_code(Code::Sbb_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Sub_rm16_imm8 => {instruction.set_code(Code::Sub_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Sub_rm32_imm8 => {instruction.set_code(Code::Sub_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Sub_rm64_imm8 => {instruction.set_code(Code::Sub_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Xor_rm16_imm8 => {instruction.set_code(Code::Xor_rm16_imm16); instruction.set_op1_kind(OpKind::Immediate16)},
        Code::Xor_rm32_imm8 => {instruction.set_code(Code::Xor_rm32_imm32); instruction.set_op1_kind(OpKind::Immediate32)},
        Code::Xor_rm64_imm8 => {instruction.set_code(Code::Xor_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        _ => {}
    }
    for op in instruction.op_kinds() {
        match op {
            OpKind::Memory => {
                if instruction.memory_displ_size() != 0 {
                    instruction.set_memory_displ_size(8);
                }
            }
            _ => {}
        }
    }
}

fn change_imm_size_small(instruction: &mut Instruction) {
    match instruction.code() {
        Code::Adc_rm16_imm16 => {instruction.set_code(Code::Adc_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::Adc_rm32_imm32 => {instruction.set_code(Code::Adc_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::Adc_rm64_imm32 => {instruction.set_code(Code::Adc_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        Code::Add_rm16_imm16 => {instruction.set_code(Code::Add_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::Add_rm32_imm32 => {instruction.set_code(Code::Add_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::Add_rm64_imm32 => {instruction.set_code(Code::Add_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        Code::And_rm16_imm16 => {instruction.set_code(Code::And_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::And_rm32_imm32 => {instruction.set_code(Code::And_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::And_rm64_imm32 => {instruction.set_code(Code::And_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        Code::Cmp_rm16_imm16 => {instruction.set_code(Code::Cmp_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::Cmp_rm32_imm32 => {instruction.set_code(Code::Cmp_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::Cmp_rm64_imm32 => {instruction.set_code(Code::Cmp_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        Code::Imul_r16_rm16_imm16 => {instruction.set_code(Code::Imul_r16_rm16_imm8); instruction.set_op2_kind(OpKind::Immediate8to16)},
        Code::Imul_r32_rm32_imm32 => {instruction.set_code(Code::Imul_r32_rm32_imm8); instruction.set_op2_kind(OpKind::Immediate8to32)},
        Code::Imul_r64_rm64_imm32 => {instruction.set_code(Code::Imul_r64_rm64_imm8); instruction.set_op2_kind(OpKind::Immediate8to64)},
        Code::Mov_r64_imm64 => {instruction.set_code(Code::Mov_rm64_imm32); instruction.set_op1_kind(OpKind::Immediate32to64)},
        Code::Or_rm16_imm16 => {instruction.set_code(Code::Or_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::Or_rm32_imm32 => {instruction.set_code(Code::Or_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::Or_rm64_imm32 => {instruction.set_code(Code::Or_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        Code::Pushq_imm32 => {instruction.set_code(Code::Pushq_imm8); instruction.set_op0_kind(OpKind::Immediate8to64)},
        Code::Sbb_rm16_imm16 => {instruction.set_code(Code::Sbb_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::Sbb_rm32_imm32 => {instruction.set_code(Code::Sbb_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::Sbb_rm64_imm32 => {instruction.set_code(Code::Sbb_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        Code::Sub_rm16_imm16 => {instruction.set_code(Code::Sub_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::Sub_rm32_imm32 => {instruction.set_code(Code::Sub_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::Sub_rm64_imm32 => {instruction.set_code(Code::Sub_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        Code::Xor_rm16_imm16 => {instruction.set_code(Code::Xor_rm16_imm8); instruction.set_op1_kind(OpKind::Immediate8to16)},
        Code::Xor_rm32_imm32 => {instruction.set_code(Code::Xor_rm32_imm8); instruction.set_op1_kind(OpKind::Immediate8to32)},
        Code::Xor_rm64_imm32 => {instruction.set_code(Code::Xor_rm64_imm8); instruction.set_op1_kind(OpKind::Immediate8to64)},
        _ => {}
    }
    for op in instruction.op_kinds() {
        match op {
            OpKind::Memory => {
                if instruction.memory_displ_size() != 0 {
                    if instruction.is_ip_rel_memory_operand() {
                        instruction.set_memory_displ_size(8);
                    } else {
                        instruction.set_memory_displ_size(1);
                    }
                }
            }
            _ => {}
        }
    }
}

fn get_instruction_imm_and_memory_offsets(instruction: &Instruction) -> (u64, u64) {
    let instruction_len = instruction.len();
    let mut imm_bytes = 0;
    let mut mem_bytes = 0;
    for op_kind in instruction.op_kinds() {
        match op_kind {
            OpKind::NearBranch64 => {
                if instruction.is_jcc_near() | instruction.is_jmp_near() | instruction.is_call_near() {
                    imm_bytes += 4
                } else {
                    imm_bytes += 1
                }
            }
            OpKind::Immediate8 |
            OpKind::Immediate8_2nd |
            OpKind::Immediate8to16 |
            OpKind::Immediate8to32 |
            OpKind::Immediate8to64 => {
                imm_bytes += 1;
            }
            OpKind::Immediate16 => {
                imm_bytes += 2;
            }
            OpKind::Immediate32 |
            OpKind::Immediate32to64 => {
                imm_bytes += 4;
            }
            OpKind::Immediate64 => {
                imm_bytes += 8;
            }
            OpKind::Memory => {
                mem_bytes += instruction.memory_displ_size().min(4) as usize;
            }
            _ => {}
        }
    }
    ((instruction_len - imm_bytes) as u64, (instruction_len - imm_bytes - mem_bytes) as u64)
}

fn get_instruction_imm_and_memory_offsets_2(instruction: &Instruction) -> (u64, u64) {
    let mut imm_bytes = 0;
    let mut mem_bytes = 0;
    for op_kind in instruction.op_kinds() {
        match op_kind {
            OpKind::NearBranch64 => {
                if instruction.is_jcc_near() | instruction.is_jmp_near() | instruction.is_call_near() {
                    imm_bytes += 4
                } else {
                    imm_bytes += 1
                }
            }
            OpKind::Immediate8 |
            OpKind::Immediate8_2nd |
            OpKind::Immediate8to16 |
            OpKind::Immediate8to32 |
            OpKind::Immediate8to64 => {
                imm_bytes += 1;
            }
            OpKind::Immediate16 => {
                imm_bytes += 2;
            }
            OpKind::Immediate32 |
            OpKind::Immediate32to64 => {
                imm_bytes += 4;
            }
            OpKind::Immediate64 => {
                imm_bytes += 8;
            }
            OpKind::Memory => {
                mem_bytes += instruction.memory_displ_size().min(4) as usize;
            }
            _ => {}
        }
    }
    (imm_bytes, imm_bytes + mem_bytes as u64)
}

fn reloc_to_immediate(
    reloc: &Relocation,
    symbols: &HashMap<usize, Symbol>,
    branch_target_table: &HashMap<u64, usize>,
    sections: &[Section],
    implicit_addend: i64,
    is_call: bool,
) -> Immediate {
    match reloc.target() {
        RelocationTarget::Symbol(sym_idx) => {
            let symbol = &symbols.get(&sym_idx.0).unwrap();
            if symbol.is_undefined() {
                match reloc.kind() {
                    RelocationKind::Absolute => {
                        return Immediate::UnresolvedSymbol(symbol.name().unwrap().to_owned(), reloc.addend() + implicit_addend);
                    }
                    RelocationKind::Relative => {
                        return Immediate::UnresolvedSymbolRel(symbol.name().unwrap().to_owned(), reloc.addend() + implicit_addend);
                    }
                    RelocationKind::PltRelative => {
                        return Immediate::UnresolvedSymbolRel(symbol.name().unwrap().to_owned(), reloc.addend() + implicit_addend);
                    }
                    RelocationKind::Unknown => {
                        return Immediate::UnresolvedSymbol(symbol.name().unwrap().to_owned(), reloc.addend() + implicit_addend);
                    }
                    _ => panic!("Unsupported reloc kind for external symbol")
                }
            } else {
                match symbol.kind() {
                    SymbolKind::Section => {
                        let section_idx = symbol.section_index().unwrap().0 - 1;
                        let section = &sections[section_idx];
                        match section.kind() {
                            SectionKind::Text => {
                                let instruction_idx = branch_target_table.get(&((reloc.addend() + implicit_addend) as u64 + 0x40_0000)).expect("Relocation not aligned to instruction boundary");
                                match reloc.kind() {
                                    RelocationKind::Absolute => {
                                        if is_call {
                                            return Immediate::InstructionOffsetCall(*instruction_idx);
                                        } else {
                                            return Immediate::InstructionOffset(*instruction_idx);
                                        }
                                    }
                                    RelocationKind::Unknown => { //untested
                                        if is_call {
                                            return Immediate::InstructionOffsetCall(*instruction_idx);
                                        } else {
                                            return Immediate::InstructionOffset(*instruction_idx);
                                        }
                                    }
                                    _ => panic!("Unsupported reloc kind for symbol in text section")
                                }
                            }
                            SectionKind::Data => {
                                match reloc.kind() {
                                    RelocationKind::Absolute => {
                                        return Immediate::DataSymbol(reloc.addend() + implicit_addend);
                                    }
                                    RelocationKind::Relative => {
                                        return Immediate::DataSymbolRel(reloc.addend() + implicit_addend);
                                    }
                                    RelocationKind::Unknown => {
                                        return Immediate::DataSymbol(reloc.addend() + implicit_addend);
                                    }
                                    _ => panic!("Unsupported reloc kind for symbol in data section")
                                }
                            }
                            _ => {panic!("Strange section for relocation: {:?}", section.kind())}
                        }
                    }
                    _ => panic!("Unknown relocation symbol to handle: {:?}", symbol.kind())
                }
            }
        }
        _ => panic!("Unimplemented relocation target")
    }
}
