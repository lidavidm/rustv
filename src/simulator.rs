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

use cache::SharedCache;
use isa;
use isa::IsaType;
use memory::{MemoryInterface, MemoryError, Mmu, SharedMemory};
use register_file::RegisterFile;
use syscall::SyscallHandler;
use trap::Trap;

pub struct Core<'a> {
    id: usize,
    pc: isa::Address,
    registers: RegisterFile,
    stall: u32,
    running: bool,
    cache: SharedCache<'a>,
    mmu: Box<Mmu + 'a>,
    cycle_count: u32,
    stall_count: u32,
}

/// Why the simulator has halted execution.
pub enum HaltReason {
    /// All cores have halted execution.
    CoresHalted,
    /// The simulator has hit the cycle limit.
    OutOfCycles,
    /// The syscall handler has requested a halt.
    SystemHalt,
}

pub struct Simulator<'a, T: SyscallHandler> {
    cores: Vec<Core<'a>>,
    memory: SharedMemory<'a>,
    caches: Vec<SharedMemory<'a>>,
    syscall: T,
}

impl<'a> Core<'a> {
    // TODO: take Rc<RefCell<>> to Memory as well?
    pub fn new(id: usize, entry: isa::Address, sp: isa::Address,
               cache: SharedCache<'a>, mmu: Box<Mmu + 'a>) -> Core<'a> {
        let mut registers = RegisterFile::new();
        registers.write_word(isa::Register::X2, sp);
        Core {
            id: id,
            pc: entry,
            registers: registers,
            stall: 0,
            running: true,
            cache: cache,
            mmu: mmu,
            cycle_count: 0,
            stall_count: 0,
        }
    }

    pub fn registers(&mut self) -> &mut RegisterFile {
        &mut self.registers
    }

    fn step(&mut self, inst: isa::Instruction, system: &mut SyscallHandler) {
        let pc = self.pc;

        self.cycle_count += 1;

        if self.stall > 0 {
            self.stall -= 1;
            self.stall_count += 1;
            return;
        }

        match inst.opcode() {
            isa::opcodes::LUI => {
                self.registers.write_word(inst.rd(), inst.u_imm().as_word())
            },
            isa::opcodes::AUIPC => {
                let result = (pc.as_signed_word()) + inst.u_imm();
                self.registers.write_word(inst.rd(), result.as_word());
            },
            isa::opcodes::JALR => {
                // TODO: assert funct3 is 0
                let base = self.registers.read_word(inst.rs1())
                   .as_signed_word();
                let target = (base + inst.i_imm()).as_address();
                let retval = (pc + 4).as_word();
                if target == isa::Word(0x0) {
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
                let target = ((pc.as_signed_word()) + inst.uj_imm()).as_address();
                self.registers.write_word(inst.rd(), (pc + 4).as_word());
                self.pc = target;
                // panic!("JAL to {:X} 0x{:X}", pc, target);
                return;
            }
            isa::opcodes::BRANCH => {
                let target = ((pc.as_signed_word()) + inst.sb_imm()).as_address();
                let rs1 = self.registers.read_word(inst.rs1());
                let rs2 = self.registers.read_word(inst.rs2());
                if match inst.funct3() {
                    isa::funct3::BEQ => rs1 == rs2,
                    isa::funct3::BNE => rs1 != rs2,
                    isa::funct3::BLT => (rs1.as_signed_word()) < (rs2.as_signed_word()),
                    isa::funct3::BGE => (rs1.as_signed_word()) >= (rs2.as_signed_word()),
                    isa::funct3::BLTU => rs1 < rs2,
                    isa::funct3::BGEU => rs1 >= rs2,
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
                let src = self.registers.read_word(inst.rs1()).as_signed_word();
                if let Some(value) = match inst.funct3() {
                    isa::funct3::ADDI => {
                        Some(src.wrapping_add(imm).as_word())
                    },
                    isa::funct3::SLLI => {
                        Some((src.as_word() << inst.shamt()))
                    },
                    isa::funct3::SLTI => {
                        if src < imm {
                            Some(isa::Word(1))
                        }
                        else {
                            Some(isa::Word(0))
                        }
                    },
                    isa::funct3::SLTIU => {
                        if (src.as_word()) < (imm.as_word()) {
                            Some(isa::Word(1))
                        }
                        else {
                            Some(isa::Word(0))
                        }
                    },
                    isa::funct3::XORI => {
                        Some((src ^ imm).as_word())
                    },
                    isa::funct3::SRLI_SRAI => {
                        match inst.funct7() {
                            isa::funct7::SRLI => Some(((src.as_word()) >> inst.shamt()).as_word()),
                            isa::funct7::SRAI => Some((src >> inst.shamt() as i32).as_word()),
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
                        Some((src | imm).as_word())
                    },
                    isa::funct3::ANDI => {
                        Some((src & imm).as_word())
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
                            isa::funct7::ADD_SRL => Some(((src1.as_signed_word()).wrapping_add(src2.as_signed_word())).as_word()),
                            isa::funct7::SUB_SRA => Some(((src1.as_signed_word()).wrapping_sub(src2.as_signed_word())).as_word()),
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
                        if (src1.as_signed_word()) < (src2.as_signed_word()) {
                            Some(isa::Word(1))
                        }
                        else {
                            Some(isa::Word(0))
                        }
                    },
                    isa::funct3::SLTU => {
                        if src1 < src2 {
                            Some(isa::Word(1))
                        }
                        else {
                            Some(isa::Word(0))
                        }
                    },
                    isa::funct3::XOR => {
                        Some(src1 ^ src2)
                    },
                    isa::funct3::SRL_SRA => {
                        match inst.funct7() {
                            isa::funct7::ADD_SRL => Some(src1 >> src2_shift),
                            isa::funct7::SUB_SRA =>
                                Some(((src1.as_signed_word()) >>
                                      src2_shift.as_signed_word()).as_word()),
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
            isa::opcodes::LOAD => {
                let imm = inst.i_imm();
                let base = self.registers.read_word(inst.rs1());
                let address = ((base.as_signed_word()) + imm).as_address();
                let address = self.mmu.translate(address);

                let result = match inst.funct3() {
                    isa::funct3::LB =>
                        self.cache.borrow_mut()
                        .read_byte(address)
                        .map(|b| b.as_signed_word().as_word()),
                    isa::funct3::LH =>
                        self.cache.borrow_mut()
                        .read_halfword(address)
                        .map(|b| b.as_signed_word().as_word()),
                    isa::funct3::LW =>
                        self.cache.borrow_mut().read_word(address),
                    isa::funct3::LBU =>
                        self.cache.borrow_mut()
                        .read_byte(address)
                        .map(|b| b.as_word()),
                    isa::funct3::LHU =>
                        self.cache.borrow_mut()
                        .read_halfword(address)
                        .map(|b| b.as_word()),
                    _ => panic!("{:x}: Invalid load funct3code: 0x{:x}",
                                pc, inst.funct3()),
                };

                match result {
                    Ok(value) => self.registers.write_word(inst.rd(), value),
                    Err(MemoryError::CacheMiss { stall_cycles, retry }) => {
                        self.stall = stall_cycles - 1;
                        if retry {
                            return;  // don't increment PC
                        }
                    },
                    Err(MemoryError::InvalidAddress) => {
                        self.trap(Trap::IllegalRead {
                            address: pc,
                            instruction: inst,
                            memory_address: address,
                        });
                    },
                }
            },
            isa::opcodes::STORE => {
                let imm = inst.s_imm();
                let base = self.registers.read_word(inst.rs1());
                let val = self.registers.read_word(inst.rs2());
                let address = ((base.as_signed_word()) + imm).as_address();
                let address = self.mmu.translate(address);

                let result = match inst.funct3() {
                    isa::funct3::SB =>
                        self.cache.borrow_mut()
                        .write_byte(address, val.as_byte()),
                    isa::funct3::SH =>
                        self.cache.borrow_mut()
                        .write_halfword(address, val.as_half_word()),
                    isa::funct3::SW =>
                        self.cache.borrow_mut().write_word(address, val),
                    _ => panic!("PC {:x}: Invalid store funct3code: 0x{:x}",
                                pc, inst.funct3()),
                };

                match result {
                    Ok(()) => (),
                    Err(MemoryError::CacheMiss { stall_cycles, retry }) => {
                        self.stall = stall_cycles - 1;
                        if retry {
                            return;  // don't increment PC
                        }
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
            },
            isa::opcodes::SYSTEM => match inst.i_imm() {
                isa::SignedWord(0x0) => {
                    let result = system.syscall(self.id, &mut self.registers,
                                                &*self.mmu);
                    if let Some(trap) = result {
                        self.trap(trap);
                    }
                }
                _ => {

                }
            },
            _ => {
                panic!("Invalid opcode: 0x{:02X} at PC 0x{:X} in instruction {:?}",
                       inst.opcode(), pc, inst);
            }
        }
        self.pc += 4;
    }

    fn trap(&mut self, trap: Trap) {
        println!("Trap: {:?}", trap);
        self.running = false;
    }
}

impl<'a, T: SyscallHandler> Simulator<'a, T> {
    pub fn new(cores: Vec<Core<'a>>, memory: SharedMemory<'a>,
               caches: Vec<SharedMemory<'a>>, syscall: T)
               -> Simulator<'a, T> {
        // TODO: initialize GP, registers (GP is in headers)
        Simulator {
            cores: cores,
            memory: memory,
            caches: caches,
            syscall: syscall,
        }
    }

    fn step(&mut self) -> bool {
        let mut ran = false;
        for core in self.cores.iter_mut() {
            if !core.running {
                continue;
            }

            let pc = core.pc;
            let pc = core.mmu.translate(pc);
            let inst = self.memory.borrow_mut().read_instruction(pc);

            if let Some(inst) = inst {
                core.step(inst, &mut self.syscall);
            }
            else {
                // TODO: trap
            }

            ran = true;
        }

        for cache in self.caches.iter() {
            cache.borrow_mut().step();
        }

        ran
    }

    pub fn report(&self) -> Vec<(usize, u32, u32)> {
        self.cores.iter()
            .map(|core| (core.id, core.stall_count, core.cycle_count))
            .collect()
    }

    pub fn run(&mut self) -> HaltReason {
        loop {
            if !self.step() {
                return HaltReason::CoresHalted;
            }
            if self.syscall.should_halt() {
                return HaltReason::SystemHalt;
            }
        }
    }

    pub fn run_max(&mut self, cycles: usize) -> HaltReason {
        for _ in 0..cycles {
            if !self.step() {
                return HaltReason::CoresHalted;
            }
            if self.syscall.should_halt() {
                return HaltReason::SystemHalt;
            }
        }

        return HaltReason::OutOfCycles;
    }
}
