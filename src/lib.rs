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
        Ok(b) => {
            let mut simulator = simulator::Simulator::new(1, b);
            simulator.run();
        },
        Err(err) => println!("Error: {:?}", err),
    }
}
