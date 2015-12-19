use isa;
use binary::{Binary};
use memory::{Memory};

pub struct Simulator {
    num_cores: usize,
    memory: Memory,
}

#[derive(Clone)]
struct RegisterFile {
    registers: [u32; 32],
}

#[derive(Clone)]
struct Core {
    pc: usize,
    registers: RegisterFile,
    running: bool,
}

#[derive(Debug)]
enum Trap {
    IllegalInstruction {
        address: usize,
        instruction: isa::Instruction,
    },
    IllegalRead {
        address: usize,
        instruction: isa::Instruction,
        memory_address: usize,
    },
    IllegalWrite {
        address: usize,
        instruction: isa::Instruction,
        memory_address: usize,
        memory_value: u32,
    }
}

impl RegisterFile {
    fn new() -> RegisterFile {
        RegisterFile {
            registers: [0; 32],
        }
    }

    fn write_word<T: Into<isa::Register>>(&mut self, reg: T, value: u32) {
        // TODO: should be safe to use unchecked index
        let reg = reg.into();
        if reg == isa::Register::X0 { return; }
        self.registers[reg.as_num()] = value;
    }

    fn read_word<T: Into<isa::Register>>(&mut self, reg: T) -> u32 {
        self.registers[reg.into().as_num()]
    }
}

impl Simulator {
    pub fn new(num_cores: usize, binary: Binary) -> Simulator {
        let memory = Memory::new(0x2000, binary);
        // TODO: initialize GP, registers (GP is in headers)
        Simulator {
            num_cores: num_cores,
            memory: memory,
        }
    }

    pub fn run(&mut self) {
        let base_core = Core {
            pc: 0x10000,
            registers: RegisterFile::new(),
            running: true,
        };
        let mut cores = vec![base_core ; self.num_cores];
        // hardcode GP
        cores[0].registers.write_word(isa::Register::X3, 0x10860);
        // hardcode SP
        cores[0].registers.write_word(isa::Register::X2, 0x7FFC);
        loop {
            let mut ran = false;
            for core in cores.iter_mut() {
                if !core.running {
                    continue;
                }
                self.step_core(core);
                ran = true;
            }
            if !ran {
                println!("All cores are not running, stopping...");
                break;
            }
        }
    }

    fn step_core(&mut self, core: &mut Core) {
        let pc = core.pc;
        if let Some(inst) = self.memory.read_instruction(pc) {
            match inst.opcode() {
                isa::opcodes::JALR => {
                    // TODO: assert funct3 is 0
                    let target = ((core.registers.read_word(inst.rs1()) as i32) + inst.i_imm()) as u32;
                    if target == 0x0 {
                        // ret
                        core.running = false;
                    }
                },
                isa::opcodes::JAL => {
                    
                }
                isa::opcodes::BRANCH => {
                    
                },
                isa::opcodes::INTEGER_IMMEDIATE => match inst.funct3() {
                    isa::funct3::ADDI => {
                        let imm = inst.i_imm();
                        let src: i32 = core.registers.read_word(inst.rs1()) as i32;
                        core.registers.write_word(inst.rd(), src.wrapping_add(imm) as u32);
                    },
                    _ => {
                        panic!("Invalid integer-immediate funct3code: 0x{:x}", inst.funct3());
                    }
                },
                isa::opcodes::INTEGER_REGISTER => {
                    let src1 = core.registers.read_word(inst.rs1());
                    let src2 = core.registers.read_word(inst.rs2());
                    let src2_shift = src2 & 0x1F;
                    if let Some(value) = match inst.funct3() {
                        isa::funct3::ADD_SUB => {
                            match inst.funct7() {
                                isa::funct7::ADD_SRL => Some(((src1 as i32).wrapping_add(src2 as i32)) as u32),
                                isa::funct7::SUB_SRA => Some(((src1 as i32).wrapping_sub(src2 as i32)) as u32),
                                _ => {
                                    self.trap(core, Trap::IllegalInstruction {
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
                            if (src1 as i32) < (src2 as i32) {
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
                                isa::funct7::SUB_SRA => Some(((src1 as i32) >> src2_shift) as u32),
                                _ => {
                                    self.trap(core, Trap::IllegalInstruction {
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
                            self.trap(core, Trap::IllegalInstruction {
                                address: pc,
                                instruction: inst,
                            });
                            None
                        }
                    } {
                        core.registers.write_word(inst.rd(), value);
                    }
                },
                isa::opcodes::LOAD => match inst.funct3() {
                     isa::funct3::LW => {
                         let imm = inst.i_imm();
                         let base = core.registers.read_word(inst.rs1());
                         let address = ((base as i32) + imm) as usize;
                         if let Some(value) = self.memory.read_word(address) {
                             core.registers.write_word(inst.rd(), value);
                             println!("Load to {:?}: 0x{:X}", inst.rd(), value);
                         }
                         else {
                             self.trap(core, Trap::IllegalRead {
                                 address: pc,
                                 instruction: inst,
                                 memory_address: address,
                             });
                         }
                     }
                    _ => {
                        panic!("Invalid load funct3code: 0x{:x}", inst.funct3());
                    }
                },
                isa::opcodes::STORE => match inst.funct3() {
                     isa::funct3::SW => {
                         let imm = inst.s_imm();
                         let base = core.registers.read_word(inst.rs1());
                         let val = core.registers.read_word(inst.rs2());
                         let address = ((base as i32) + imm) as usize;
                         self.memory.write_word(address, val);
                         println!("Store to 0x{:X}: 0x{:X}", address, val);
                    }
                    _ => {
                        panic!("Invalid store funct3code: 0x{:x}", inst.funct3());
                    }
                },
                isa::opcodes::SYSTEM => {
                    
                },
                _ => {
                    panic!("Invalid opcode: 0x{:02X}", inst.opcode());
                }
            }
        }
        else {
            // trap
        }
        core.pc += 4;
    }

    fn trap(&mut self, core: &mut Core, trap: Trap) {
        println!("Trap: {:?}", trap);
        core.running = false;
    }
}
