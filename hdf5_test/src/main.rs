use std::fs;
use std::path::PathBuf;

fn golden(name: &str) -> Vec<u8> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("..");
    p.push("crates");
    p.push("tpt-io-netcdf");
    p.push("tests");
    p.push("samples");
    p.push(name);
    fs::read(&p).unwrap()
}

fn main() {
    let data = golden("example4.h5");
    match hdf5_io::file::File::from_bytes(data.into_boxed_slice()) {
        Ok(f) => {
                match f.root_group() {
                    Ok(root) => {
                        println!("Root group members: {:?}", root.members());
                    }
                Err(e) => {
                    println!("Root group error: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
