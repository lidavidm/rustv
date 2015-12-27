#![feature(associated_consts)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_address_parsing() {
        let dm_cache_word = memory::Cache::new(4, 1, 1);
        let dm_cache_doubleword = memory::Cache::new(4, 1, 2);
        let fa_cache_doubleword = memory::Cache::new(1, 4, 2);

        assert_eq!(dm_cache_word.parse_address(0xFFFFFFFD),
                   (0xFFFFFFF, 3, 1));
        assert_eq!(dm_cache_doubleword.parse_address(0xFFFFFFFD),
                   (0x7FFFFFF, 3, 5));
        assert_eq!(fa_cache_doubleword.parse_address(0xFFFFFFFD),
                   (0x1FFFFFFF, 0, 5));
    }
}
