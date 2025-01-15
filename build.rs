//use std::process::Command;
use std::env;
//use std::str;
use std::fs;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    println!("cargo::rerun-if-changed=src/asm/");
    
    for file in fs::read_dir(manifest_dir.to_owned()+"/src/asm/").expect("Failed to read src/asm") {
        let path = file
            .unwrap()
            .path();
        
        if let Some(extension) = path.extension() {
            if extension == "s" {
                
                let path_vec = cc::Build::new()
                    .asm_flag("-fPIC")
                    .file(path.clone())
                    .debug(true)
                    .compile_intermediates();
                
                fs::rename(
                    path_vec[0].clone(),
                    format!("{}/{}.o",
                        out_dir,
                        path
                            .as_path()
                            .file_stem()
                            .unwrap()
                            .to_str()
                            .unwrap()
                    )
                ).expect("Failed to rename file");
            }
        }
    }
    
}

