// Copyright 2015-2016 David Li
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

#![feature(augmented_assignments, braced_empty_structs,
           op_assign_traits, step_by)]
extern crate elfloader32 as elfloader_lib;

pub mod isa;
pub mod memory;
pub mod register_file;
pub mod simulator;
pub mod syscall;
pub mod trap;

pub use elfloader_lib as elfloader;

#[cfg(test)]
mod tests {
    #[test]
    fn cache_address_parsing() {
        use memory::*;
        use std::rc::Rc;
        use std::cell::RefCell;

        let memory = Memory::new(16);
        let memory_ref = Rc::new(RefCell::new(memory));
        let dm_cache_word = DirectMappedCache::new(4, 1, memory_ref.clone());
        let dm_cache_doubleword = DirectMappedCache::new(4, 2, memory_ref.clone());

        assert_eq!(dm_cache_word.parse_address(0xFFFFFFFD),
                   (0xFFFFFFF, 3, 1));
        assert_eq!(dm_cache_doubleword.parse_address(0xFFFFFFFD),
                   (0x7FFFFFF, 3, 5));
    }

    #[test]
    fn memory_rw() {
        use std::rc::Rc;
        use std::cell::RefCell;
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
        assert_eq!(memory.write_byte(3, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_halfword(0, 0xF0),
                   Err(MemoryError::InvalidAddress));
        assert_eq!(memory.write_halfword(2, 0xF0),
                   Err(MemoryError::InvalidAddress));

        for address in (4..size).step_by(4) {
            assert_eq!(memory.write_word(address, 0xF0), Ok(()));
            assert_eq!(memory.read_word(address), Ok(0xF0));
            assert_eq!(memory.read_halfword(address), Ok(0xF0));
            assert_eq!(memory.read_halfword(address + 2), Ok(0x0));
            assert_eq!(memory.read_byte(address), Ok(0xF0));
            assert_eq!(memory.read_byte(address + 1), Ok(0x0));
            assert_eq!(memory.read_byte(address + 2), Ok(0x0));
            assert_eq!(memory.read_byte(address + 3), Ok(0x0));
        }

        assert_eq!(memory.write_word(0x10, 0x01234567), Ok(()));
        assert_eq!(memory.write_word(0x14, 0xDEADBEEF), Ok(()));
        assert_eq!(memory.read_byte(0x10), Ok(0x67));
        assert_eq!(memory.read_byte(0x11), Ok(0x45));
        assert_eq!(memory.read_byte(0x12), Ok(0x23));
        assert_eq!(memory.read_byte(0x13), Ok(0x01));
        assert_eq!(memory.read_halfword(0x10), Ok(0x4567));
        assert_eq!(memory.read_halfword(0x12), Ok(0x0123));

        let stall = Err(MemoryError::CacheMiss {
            stall_cycles: memory.latency(),
        });
        let write_stall = Err(MemoryError::CacheMiss {
            stall_cycles: memory.latency(),
        });

        let memory_ref = Rc::new(RefCell::new(memory));
        let mut dm_cache = DirectMappedCache::new(4, 4, memory_ref.clone());

        assert_eq!(dm_cache.read_word(0x10), stall);

        for _ in 0..100 {
            dm_cache.step();
        }

        assert_eq!(dm_cache.write_word(0x20, 0x123), write_stall);
        assert_eq!(dm_cache.read_word(0x10), Ok(0x01234567));
        assert_eq!(dm_cache.read_word(0x14), Ok(0xDEADBEEF));
        assert_eq!(dm_cache.read_word(0x18), Ok(0xF0));
        assert_eq!(dm_cache.read_word(0x1C), Ok(0xF0));
        assert_eq!(dm_cache.read_byte(0x10), Ok(0x67));
        assert_eq!(dm_cache.read_byte(0x11), Ok(0x45));
        assert_eq!(dm_cache.read_byte(0x12), Ok(0x23));
        assert_eq!(dm_cache.read_byte(0x13), Ok(0x01));
        assert_eq!(dm_cache.write_word(0x18, 0xBEEFBEEF), Ok(()));
        assert_eq!(dm_cache.read_word(0x18), Ok(0xBEEFBEEF));
        assert_eq!(dm_cache.read_halfword(0x10), Ok(0x4567));
        assert_eq!(dm_cache.read_halfword(0x12), Ok(0x0123));
        assert_eq!(memory_ref.borrow_mut().read_word(0x18), Ok(0xBEEFBEEF));

        for _ in 0..100 {
            dm_cache.step();
        }

        assert_eq!(dm_cache.write_word(0x20, 0x123), Ok(()));
        assert_eq!(memory_ref.borrow_mut().read_word(0x20), Ok(0x123));
        // Should not have been evicted
        assert_eq!(dm_cache.read_word(0x10), Ok(0x01234567));
        assert_eq!(dm_cache.read_word(0x14), Ok(0xDEADBEEF));
        assert_eq!(dm_cache.read_word(0x18), Ok(0xBEEFBEEF));
        assert_eq!(dm_cache.read_word(0x1C), Ok(0xF0));
        assert_eq!(dm_cache.read_halfword(0x10), Ok(0x4567));
        assert_eq!(dm_cache.read_halfword(0x12), Ok(0x0123));

        assert_eq!(dm_cache.write_byte(0x10, 0x42), Ok(()));
        assert_eq!(dm_cache.write_halfword(0x12, 0x4242), Ok(()));
        assert_eq!(memory_ref.borrow_mut().read_word(0x10), Ok(0x42424542));
    }
}
