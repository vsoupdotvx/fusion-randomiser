#![cfg_attr(target_os = "linux", feature(unix_socket_ancillary_data))]
#![cfg_attr(target_os = "linux", feature(anonymous_pipe))]
use std::{collections::HashMap, env, path::PathBuf, str::FromStr, sync::Arc, thread::sleep, time::Duration};

use data::init_defaults_from_dump;
use il2cppdump::IL2CppDumper;
use logic::RandomisationData;
use patcher::Patch;
use process::FusionProcess;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use util::{hash_str, CommonError};

pub mod il2cppdump;
pub mod patcher;
pub mod process;
pub mod util;
pub mod data;
pub mod logic;

#[derive(Debug)]
enum ArgType {
    None,
    Seed,
    OutDir,
}

fn main() {
    let mut seed_rng = ChaCha8Rng::from_os_rng();
    let mut seed_string = seed_rng.next_u64().to_string();
    let mut connect = true;
    let mut restrictions = true;
    let mut out_dir: Option<String> = None;
    
    let mut arg_type = ArgType::None;
    for arg in env::args().skip(1) {
        match arg_type {
            ArgType::OutDir => {
                out_dir = Some(arg);
                arg_type = ArgType::None;
            }
            ArgType::Seed   => {
                seed_string = arg;
                arg_type = ArgType::None;
            }
            ArgType::None   => match arg.as_str() {
                "--no-connect" => connect = false,
                "-R" => restrictions = false,
                "-o" => arg_type = ArgType::OutDir,
                "-s" => arg_type = ArgType::Seed,
                _ => panic!("Unknown argument \"{arg}\"")
            }
        }
    }
    match arg_type {
        ArgType::None => {}
        _ => panic!("{arg_type:?} requires an argument!")
    }
    
    let seed = hash_str(&seed_string);
    println!("Seed: \"{seed_string}\"");
    
    let base_patch      = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/base.o")));
    let tutorials_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/tutorials.o")));
    let firerates_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/firerates.o")));
    let cost_patch      = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cost.o")));
    let cooldowns_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cooldowns.o")));
    let spawns_patch    = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/spawns.o")));
    
    let mut fusion = FusionProcess::new(connect).unwrap();
    let mut dumper = IL2CppDumper::initialize(&fusion.files_dir).unwrap();
    
    if let Some(out_dir) = out_dir {
        let out_path = match PathBuf::from_str(&out_dir) {
            Ok(path) => path,
            Err(err) => panic!("Error while parsing path: {err}")
        };
        if out_path.is_dir() {
            let dump_arc = Arc::new(dumper);
            
            match dump_arc.output_disasm(out_path.clone().join("out.s")) {
                Ok(())  => println!("Successfully output disasm"),
                Err(()) => println!("Failed to output disasm"),
            }
            
            match dump_arc.output_structs(out_path.clone().join("out_structs.rs")) {
                Ok(())  => println!("Successfully output structs"),
                Err(()) => println!("Failed to output structs"),
            }
            
            dumper = Arc::into_inner(dump_arc).unwrap();
        } else {
            panic!("Output path \"{}\" is not a directory!", out_path.to_string_lossy());
        }
    }
    
    init_defaults_from_dump(&dumper);
    let sym_tab = Patch::apply_patches(&[
        base_patch.unwrap(),
        tutorials_patch.unwrap(),
        firerates_patch.unwrap(),
        cost_patch.unwrap(),
        cooldowns_patch.unwrap(),
        spawns_patch.unwrap(),
    ], &dumper, &mut fusion);
    println!("Game patched!");
    
    let mut level_idx = 0;
    
    let level_addr    = *sym_tab.get("level_idx").unwrap();
    let wait_addr     = *sym_tab.get("stopped").unwrap();
    let mix_ptr_addr  = *sym_tab.get("mix_data_ptr").unwrap();
    //let game_ptr_addr = *sym_tab.get("game_app_ptr").unwrap();
    
    let mut initialized  = false;
    let mut mem_read_vec = Vec::new();
    let mut fuse_map: HashMap<u32,[u32;2]> = HashMap::new();
    let mut rand_data: Option<RandomisationData> = None;
    loop {
        sleep(Duration::from_millis(10));
        if let Err(err) = fusion.read_memory(wait_addr, 1, &mut mem_read_vec) {
            match err.downcast::<CommonError>() {
                Err(err) => panic!("Failed to read memory: {err}"),
                _ => break, //if a CommonError is returned, it means fusion closed
            }
        }
        if mem_read_vec[0] == 0 {
            continue;
        }
        
        fusion.read_memory(level_addr, 4, &mut mem_read_vec).unwrap();
        level_idx = u32::from_le_bytes(mem_read_vec[0..4].try_into().unwrap());
        
        if !initialized {
            //fusion.read_memory(game_ptr_addr, 8, &mut mem_read_vec).unwrap();
            //let game_app_addr = u64::from_le_bytes(mem_read_vec[0..8].try_into().unwrap());
            
            //fusion.read_memory(, , )
            
            fusion.read_memory(mix_ptr_addr, 8, &mut mem_read_vec).unwrap();
            let mix_data_addr = u64::from_le_bytes(mem_read_vec[0..8].try_into().unwrap());
            
            fusion.read_memory(mix_data_addr + 0x10, 8, &mut mem_read_vec).unwrap();
            let mix_array_addr = u64::from_le_bytes(mem_read_vec[0..8].try_into().unwrap());
            
            fusion.read_memory(mix_array_addr, 0x20, &mut mem_read_vec).unwrap();
            let mix_array_width = u32::from_le_bytes(mem_read_vec[0x18..0x1C].try_into().unwrap()) as usize;
            
            fusion.read_memory(mix_array_addr + 0x20, mix_array_width * 8, &mut mem_read_vec).unwrap();
            
            let mut tmp_read_vec = Vec::new();
            for (i, bytes) in mem_read_vec.chunks_exact(8).enumerate() {
                let mix_ptr = u64::from_le_bytes(bytes.try_into().unwrap());
                if mix_ptr != 0 {
                    fusion.read_memory(mix_ptr + 0x10, 8, &mut tmp_read_vec).unwrap();
                    let plant_1 = u32::from_le_bytes(tmp_read_vec[0..4].try_into().unwrap());
                    let plant_2 = u32::from_le_bytes(tmp_read_vec[4..8].try_into().unwrap());
                    if plant_1 as usize != i && plant_2 as usize != i {
                        //println!("{i}: {} + {}", plant_1, plant_2);
                        fuse_map.insert(i as u32, [plant_1, plant_2]);
                    }
                }
            }
            
            rand_data = Some(if restrictions {
                RandomisationData::restrictions(seed, &dumper, &fuse_map)
            } else {
                RandomisationData::no_restrictions(seed, &dumper, &fuse_map)
            });
            
            println!("Level order: {:?}", rand_data.as_ref().unwrap().level_order);
            
            fusion.write_memory(
                *sym_tab.get("level_lut").unwrap(),
                &unsafe { rand_data.as_ref().unwrap_unchecked() }.level_order,
            ).unwrap();
            
            fusion.write_memory(
                *sym_tab.get("plant_lut").unwrap(),
                &unsafe { rand_data.as_ref().unwrap_unchecked() }.plant_order,
            ).unwrap();
            initialized = true;
        }
        
        {
            let rand_data = unsafe { rand_data.as_ref().unwrap_unchecked() };
            if let Some(cooldowns) = &rand_data.cooldowns {
                fusion.write_memory(*sym_tab.get("plant_cd_table").unwrap(), &cooldowns[level_idx as usize]).unwrap();
            }
            if let Some(costs) = &rand_data.costs {
                fusion.write_memory(*sym_tab.get("plant_cost_table").unwrap(), &costs[level_idx as usize]).unwrap();
            }
            if let Some(spawns) = &rand_data.spawns {
                fusion.write_memory(*sym_tab.get("zombie_spawn_bitfield").unwrap(), &spawns[level_idx as usize]).unwrap();
            }
            if let Some(weights) = &rand_data.weights {
                fusion.write_memory(*sym_tab.get("zombie_weights").unwrap(), &weights[level_idx as usize]).unwrap();
            }
            if let Some(firerates) = &rand_data.firerates {
                fusion.write_memory(*sym_tab.get("plant_firerate_table").unwrap(), &firerates[level_idx as usize]).unwrap();
            }
        }
        
        fusion.write_memory(wait_addr, &[0]).unwrap();
    }
    println!("Closed on level {}", level_idx + 1);
}
