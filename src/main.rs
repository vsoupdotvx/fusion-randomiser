#![cfg_attr(target_os = "linux", feature(unix_socket_ancillary_data))]
#![feature(anonymous_pipe)]
use std::{env, thread::sleep, time::Duration};

use il2cppdump::IL2CppDumper;
use patcher::Patch;
use process::FusionProcess;

pub mod il2cppdump;
pub mod patcher;
pub mod process;
pub mod util;

fn main() {
    let mut connect = true;
    for arg in env::args() {
        match arg.as_str() {
            "--no-connect" => connect = false,
            _ => {}
        }
    }
    
    let base_patch      = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/base.o")));
    let firerates_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/firerates.o")));
    let cost_patch      = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cost.o")));
    let cooldowns_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cooldowns.o")));
    
    let mut fusion = FusionProcess::new(connect).unwrap();
    let dumper     = IL2CppDumper::initialize(&fusion.files_dir).unwrap();
    let sym_tab    = Patch::apply_patches(&[base_patch.unwrap(), firerates_patch.unwrap(), cost_patch.unwrap(), cooldowns_patch.unwrap()], &dumper, &mut fusion);
    
    let wait_addr  = *sym_tab.get("stopped").unwrap();
    let mut mem_read_vec = Vec::new();
    loop {
        sleep(Duration::from_millis(10));
        fusion.read_memory(wait_addr, 1, &mut mem_read_vec);
        if mem_read_vec[0] == 0 {
            continue;
        }
    }
}
