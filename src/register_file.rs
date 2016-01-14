// Copyright 2016 David Li
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

use isa;

pub struct RegisterFile {
    registers: [isa::Word; 32],
}

impl RegisterFile {
    pub fn new() -> RegisterFile {
        RegisterFile {
            registers: [isa::Word(0); 32],
        }
    }

    pub fn write_word<T: Into<isa::Register>>(&mut self, reg: T, value: isa::Word) {
        // TODO: should be safe to use unchecked index
        let reg = reg.into();
        if reg == isa::Register::X0 { return; }
        self.registers[reg.as_num()] = value;
    }

    pub fn read_word<T: Into<isa::Register>>(&mut self, reg: T) -> isa::Word {
        self.registers[reg.into().as_num()]
    }
}
