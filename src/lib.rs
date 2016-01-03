// Copyright 2015 David Li
// This file is part of rustv.

// rustv is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// rustv is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with rustv.  If not, see <http://www.gnu.org/licenses/>.

#![feature(step_by)]
pub mod isa;
pub mod binary;
pub mod memory;
pub mod simulator;

#[test]
fn it_works() {
    use std::rc::Rc;
    use std::cell::RefCell;

    use std::path::Path;
    match binary::Binary::new_from_hex_file(Path::new("../riscv/kernel.hex")) {
        Ok(b) => {
            let memory = memory::Memory::new_from_binary(0x2000, b);
            let memory_ref = Rc::new(RefCell::new(Box::new(memory) as Box<memory::MemoryInterface>));
            let cache = Rc::new( RefCell::new( Box::new( memory::DirectMappedCache::new(4, 4, memory_ref.clone())) as Box<memory::MemoryInterface>) );
            let core = simulator::Core::new(cache.clone());
            let mut simulator = simulator::Simulator::new(vec![core], memory_ref.clone());
            simulator.run();
        },
        Err(err) => println!("Error: {:?}", err),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cache_address_parsing() {
        use memory::*;
        use std::rc::Rc;
        use std::cell::RefCell;

        let memory = Memory::new(16);
        let memory_ref = Rc::new(RefCell::new(Box::new(memory) as Box<MemoryInterface>));
        let dm_cache_word = DirectMappedCache::new(4, 1, memory_ref.clone());
        let dm_cache_doubleword = DirectMappedCache::new(4, 2, memory_ref.clone());

        assert_eq!(dm_cache_word.parse_address(0xFFFFFFFD),
                   (0xFFFFFFF, 3, 1));
        assert_eq!(dm_cache_doubleword.parse_address(0xFFFFFFFD),
                   (0x7FFFFFF, 3, 5));
    }

    #[test]
    fn memory_rw() {
        use memory::*;
        let size = 0xFF;
        let mut memory = Memory::new(size);

        assert_eq!(memory.write_word(0, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(0, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(1, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_byte(2, 0xF0),
                   Err(MemoryError::InvalidAddress));

        for address in (4..size).step_by(4) {
            assert_eq!(memory.write_word(address, 0xF0), Ok(()));
            assert_eq!(memory.read_word(address), Ok(0xF0));
        }
    }
}
