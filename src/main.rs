use il2cppdump::IL2CppDumper;

pub mod il2cppdump;

fn main() {
    //println!("Hello, world!");
    let _ = IL2CppDumper::initialize("/home/soup/notdownloads/Fusion/".into()).unwrap();
    let _ = include_bytes!(concat!(env!("OUT_DIR"), "/test.o"));
}
