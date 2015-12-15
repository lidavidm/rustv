#![feature(as_slice)]
mod isa;
mod binary;
mod memory;
mod cache;
mod simulator;

#[test]
fn it_works() {
    use std::path::Path;
    println!("Test");
    match binary::Binary::new_from_hex_file(Path::new("../riscv/kernel.hex")) {
        Ok(_) => println!("Ok"),
        Err(err) => println!("Error: {:?}", err),
    }
}
