#![feature(unix_socket_ancillary_data)]
#![feature(anonymous_pipe)]
use std::env;

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
    
    //let _base_patch_win = Patch::new(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/base.obj")));
    let base_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/base.o")));
    let firerates_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/firerates.o")));
    let cost_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cost.o")));
    let cooldowns_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/cooldowns.o")));
    //let _test_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/test.o")));
    //println!("{_base_patch:#?}");
    let mut fusion = FusionProcess::new(connect).unwrap();
    let dumper = IL2CppDumper::initialize(&fusion.files_dir).unwrap();
    let _sym_tab = Patch::apply_patches(&[base_patch.unwrap(), firerates_patch.unwrap(), cost_patch.unwrap(), cooldowns_patch.unwrap()], &dumper, &mut fusion);
}
