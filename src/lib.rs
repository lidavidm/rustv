pub mod isa;
pub mod binary;
pub mod memory;
pub mod simulator;

#[test]
fn it_works() {
    use std::path::Path;
    match binary::Binary::new_from_hex_file(Path::new("../riscv/kernel.hex")) {
        Ok(b) => {
            let mut simulator = simulator::Simulator::new(1, b);
            simulator.run();
        },
        Err(err) => println!("Error: {:?}", err),
    }
}
