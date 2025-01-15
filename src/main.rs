#![feature(unix_socket_ancillary_data)]
#![feature(anonymous_pipe)]
use il2cppdump::IL2CppDumper;
use patcher::Patch;
use process::FusionProcess;

pub mod il2cppdump;
pub mod patcher;
pub mod process;
pub mod util;

fn main() {
    let _base_patch_win = Patch::new(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/base.obj")));
    let _base_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/base.o")));
    let _firerates_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/firerates.o")));
    let _test_patch = Patch::new(include_bytes!(concat!(env!("OUT_DIR"), "/test.o")));
    //println!("{_base_patch:#?}");
    let mut fusion = FusionProcess::new().unwrap();
    let dumper = IL2CppDumper::initialize(&fusion.files_dir).unwrap();
    let _sym_tab = Patch::apply_patches(&[_base_patch.unwrap(), _firerates_patch.unwrap()], &dumper, &mut fusion);
}
