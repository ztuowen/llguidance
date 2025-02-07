use std::env;

fn main() {
    println!("cargo:rerun-if-changed=llguidance.h");

    let copy_path = format!("{}/../../../llguidance.h", env::var("OUT_DIR").unwrap());

    std::fs::copy("llguidance.h", copy_path).unwrap();
}
