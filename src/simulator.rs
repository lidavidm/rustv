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
use memory::{MemoryInterface, MemoryError, Mmu, SharedMemory};

struct RegisterFile {
    registers: [isa::Word; 32],
}

pub struct Core<'a>{
    pc: isa::Address,
    registers: RegisterFile,
    stall: u32,
    running: bool,
    cache: SharedMemory<'a>,
    mmu: Box<Mmu + 'a>,
}

pub struct Simulator<'a> {
    cores: Vec<Core<'a>>,
    memory: SharedMemory<'a>,
}

#[derive(Debug)]
enum Trap {
    IllegalInstruction {
        address: isa::Address,
        instruction: isa::Instruction,
    },
    IllegalRead {
        address: isa::Address,
        instruction: isa::Instruction,
        memory_address: isa::Address,
    },
    IllegalWrite {
        address: isa::Address,
        instruction: isa::Instruction,
        memory_address: isa::Address,
        memory_value: isa::Word,
    }
}

impl RegisterFile {
    fn new() -> RegisterFile {
        RegisterFile {
            registers: [0; 32],
        }
    }

    fn write_word<T: Into<isa::Register>>(&mut self, reg: T, value: isa::Word) {
        // TODO: should be safe to use unchecked index
        let reg = reg.into();
        if reg == isa::Register::X0 { return; }
        self.registers[reg.as_num()] = value;
    }

    fn read_word<T: Into<isa::Register>>(&mut self, reg: T) -> isa::Word {
        self.registers[reg.into().as_num()]
    }
}

impl<'a> Core<'a> {
    // TODO: take Rc<RefCell<>> to Memory as well?
    pub fn new(cache: SharedMemory<'a>, mmu: Box<Mmu + 'a>) -> Core<'a> {
        Core {
            pc: 0x1002c, // TODO: hardcoded: fix later
            registers: RegisterFile::new(),
            stall: 0,
            running: true,
            cache: cache,
            mmu: mmu,
        }
    }

    fn step(&mut self, inst: isa::Instruction) {
        let pc = self.pc;

        if self.stall > 0 {
            self.stall -= 1;
            return;
        }

        match inst.opcode() {
            isa::opcodes::JALR => {
                // TODO: assert funct3 is 0
                let base = self.registers.read_word(inst.rs1())
                    as isa::SignedWord;
                let target = (base + inst.i_imm()) as isa::Address;
                let retval = (pc + 4) as isa::Word;
                if target == 0x0 {
                    // ret
                    self.running = false;
                }
                else {
                    self.registers.write_word(inst.rd(), retval);
                    self.pc = target;
                    return;
                }
            },
            isa::opcodes::JAL => {
                let target = ((pc as isa::SignedWord) + inst.uj_imm()) as isa::Address;
                self.registers.write_word(inst.rd(), (pc + 4) as isa::Word);
                self.pc = target;
                // panic!("JAL to {:X} 0x{:X}", pc, target);
                return;
            }
            isa::opcodes::BRANCH => {
                let target = ((pc as isa::SignedWord) + inst.sb_imm()) as isa::Address;
                let rs1 = self.registers.read_word(inst.rs1());
                let rs2 = self.registers.read_word(inst.rs2());
                if match inst.funct3() {
                    isa::funct3::BEQ => rs1 == rs2,
                    isa::funct3::BNE => rs1 != rs2,
                    isa::funct3::BLT => (rs1 as isa::SignedWord) < (rs2 as isa::SignedWord),
                    isa::funct3::BGE => (rs1 as isa::SignedWord) > (rs2 as isa::SignedWord),
                    isa::funct3::BLTU => rs1 < rs2,
                    isa::funct3::BGEU => rs1 > rs2,
                    _ => {
                        self.trap(Trap::IllegalInstruction {
                            address: pc,
                            instruction: inst,
                        });
                        false
                    }
                } {
                    self.pc = target;
                    return;
                }
            },
            isa::opcodes::INTEGER_IMMEDIATE => {
                let imm = inst.i_imm();
                let src = self.registers.read_word(inst.rs1()) as isa::SignedWord;
                if let Some(value) = match inst.funct3() {
                    isa::funct3::ADDI => {
                        Some(src.wrapping_add(imm) as isa::Word)
                    },
                    isa::funct3::SLLI => {
                        Some((src << inst.shamt()) as isa::Word)
                    },
                    isa::funct3::SLTI => {
                        if src < imm {
                            Some(1)
                        }
                        else {
                            Some(0)
                        }
                    },
                    isa::funct3::SLTIU => {
                        if (src as isa::Word) < (imm as isa::Word) {
                            Some(1)
                        }
                        else {
                            Some(0)
                        }
                    },
                    isa::funct3::XORI => {
                        Some((src ^ imm) as isa::Word)
                    },
                    isa::funct3::SRLI_SRAI => {
                        match inst.funct7() {
                            isa::funct7::SRLI => Some(((src as isa::Word) >> inst.shamt()) as isa::Word),
                            isa::funct7::SRAI => Some((src >> inst.shamt()) as isa::Word),
                            _ => {
                                self.trap(Trap::IllegalInstruction {
                                    address: pc,
                                    instruction: inst,
                                });
                                None
                            }
                        }
                    },
                    isa::funct3::ORI => {
                        Some((src | imm) as isa::Word)
                    },
                    isa::funct3::ANDI => {
                        Some((src & imm) as isa::Word)
                    },
                    _ => {
                        self.trap(Trap::IllegalInstruction {
                            address: pc,
                            instruction: inst,
                        });
                        None
                    }
                } {
                    self.registers.write_word(inst.rd(), value);
                }
            },
            isa::opcodes::INTEGER_REGISTER => {
                let src1 = self.registers.read_word(inst.rs1());
                let src2 = self.registers.read_word(inst.rs2());
                let src2_shift = src2 & 0x1F;
                if let Some(value) = match inst.funct3() {
                    isa::funct3::ADD_SUB => {
                        match inst.funct7() {
                            isa::funct7::ADD_SRL => Some(((src1 as isa::SignedWord).wrapping_add(src2 as isa::SignedWord)) as isa::Word),
                            isa::funct7::SUB_SRA => Some(((src1 as isa::SignedWord).wrapping_sub(src2 as isa::SignedWord)) as isa::Word),
                            _ => {
                                self.trap(Trap::IllegalInstruction {
                                    address: pc,
                                    instruction: inst,
                                });
                                None
                            }
                        }
                    },
                    isa::funct3::SLL => {
                        Some(src1 << src2_shift)
                    },
                    isa::funct3::SLT => {
                        if (src1 as isa::SignedWord) < (src2 as isa::SignedWord) {
                            Some(1)
                        }
                        else {
                            Some(0)
                        }
                    },
                    isa::funct3::SLTU => {
                        if src1 < src2 {
                            Some(1)
                        }
                        else {
                            Some(0)
                        }
                    },
                    isa::funct3::XOR => {
                        Some(src1 ^ src2)
                    },
                    isa::funct3::SRL_SRA => {
                        match inst.funct7() {
                            isa::funct7::ADD_SRL => Some(src1 >> src2_shift),
                            isa::funct7::SUB_SRA => Some(((src1 as isa::SignedWord) >> src2_shift) as isa::Word),
                            _ => {
                                self.trap(Trap::IllegalInstruction {
                                    address: pc,
                                    instruction: inst,
                                });
                                None
                            }
                        }
                    },
                    isa::funct3::OR => {
                        Some(src1 | src2)
                    },
                    isa::funct3::AND => {
                        Some(src1 & src2)
                    },
                    _ => {
                        self.trap(Trap::IllegalInstruction {
                            address: pc,
                            instruction: inst,
                        });
                        None
                    }
                } {
                    self.registers.write_word(inst.rd(), value);
                }
            },
            isa::opcodes::LOAD => match inst.funct3() {
                isa::funct3::LW => {
                    let imm = inst.i_imm();
                    let base = self.registers.read_word(inst.rs1());
                    let address = ((base as isa::SignedWord) + imm) as isa::Address;
                    let address = self.mmu.translate(address);

                    let result = self.cache.borrow_mut().read_word(address);
                    match result {
                        Ok(value) =>
                            self.registers.write_word(inst.rd(), value),
                        Err(MemoryError::CacheMiss { stall_cycles }) => {
                            self.stall = stall_cycles;
                            return;
                        },
                        Err(MemoryError::InvalidAddress) => {
                            self.trap(Trap::IllegalRead {
                                address: pc,
                                instruction: inst,
                                memory_address: address,
                            });
                        }
                    }
                },
                _ => {
                    panic!("Invalid load funct3code: 0x{:x}", inst.funct3());
                }
            },
            isa::opcodes::STORE => match inst.funct3() {
                isa::funct3::SW => {
                    let imm = inst.s_imm();
                    let base = self.registers.read_word(inst.rs1());
                    let val = self.registers.read_word(inst.rs2());
                    let address = ((base as isa::SignedWord) + imm) as isa::Address;
                    let address = self.mmu.translate(address);

                    let result = self.cache.borrow_mut().write_word(address, val);
                    match result {
                        Ok(()) => (),
                        Err(MemoryError::CacheMiss { stall_cycles }) => {
                            self.stall = stall_cycles;
                            return;
                        },
                        Err(MemoryError::InvalidAddress) => {
                            self.trap(Trap::IllegalWrite {
                                address: pc,
                                instruction: inst,
                                memory_address: address,
                                memory_value: val,
                            })
                        }
                    }
                }
                _ => {
                    panic!("Invalid store funct3code: 0x{:x}", inst.funct3());
                }
            },
            isa::opcodes::SYSTEM => match inst.i_imm() {
                0x0 => {
                    // System call
                    println!("System call {}:", self.registers.read_word(isa::Register::X10));
                }
                _ => {

                }
            },
            _ => {
                panic!("Invalid opcode: 0x{:02X} at PC 0x{:X}", inst.opcode(), pc);
            }
        }
        self.pc += 4;
    }

    fn trap(&mut self, trap: Trap) {
        println!("Trap: {:?}", trap);
        self.running = false;
    }
}

impl<'a> Simulator<'a> {
    pub fn new(cores: Vec<Core<'a>>, memory: SharedMemory<'a>)
               -> Simulator<'a> {
        // TODO: pass in GP, SP, _start
        // TODO: initialize GP, registers (GP is in headers)
        Simulator {
            cores: cores,
            memory: memory,
        }
    }

    pub fn run(&mut self) {
        // hardcode _start
        self.cores[0].pc = 0x1002C;
        // hardcode GP
        self.cores[0].registers.write_word(isa::Register::X3, 0x108D0);
        // hardcode SP
        self.cores[0].registers.write_word(isa::Register::X2, 0x7FFC);
        let mut total_cycles = 0;
        let mut stall_cycles = 0;
        loop {
            let mut ran = false;
            total_cycles += 1;
            for core in self.cores.iter_mut() {
                if !core.running {
                    continue;
                }
                if core.stall > 0 { stall_cycles += 1; }

                let pc = core.pc;
                let pc = core.mmu.translate(pc);
                let inst = self.memory.borrow_mut().read_instruction(pc);

                if let Some(inst) = inst {
                    core.step(inst);
                }
                else {
                    // TODO: trap
                }

                core.cache.borrow_mut().step();
                ran = true;
            }
            if !ran {
                println!("All cores are not running, stopping...");
                println!("Stalled cycles: {} of {}", stall_cycles, total_cycles);
                break;
            }
        }
    }
}
