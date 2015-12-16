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
        let mut cores = vec![Core { pc: 0x10000, registers: RegisterFile::new() }; self.num_cores];
        cores[0].registers.write_word(isa::Register::X3, 0x10860);
        loop {
            for core in cores.iter_mut() {
                self.step_core(core);
            }
        }
    }

    fn step_core(&mut self, core: &mut Core) {
        if let Some(inst) = self.memory.read_instruction(core.pc) {
            match inst.opcode() {
                isa::opcodes::BRANCH => {
                    
                },
                isa::opcodes::INTEGER_IMMEDIATE => match inst.funct3() {
                    isa::funct3::ADDI => {
                        let imm = inst.i_imm();
                        let src: i32 = core.registers.read_word(inst.rs1()) as i32;
                        core.registers.write_word(inst.rd(), src.wrapping_add(imm) as u32);
                        println!("After ADDI: {:?} = 0x{:X}", inst.rd(), core.registers.read_word(inst.rd()) as i32);
                    }
                    _ => {
                        panic!("Invalid integer-immediate funct3code: 0x{:x}", inst.funct3());
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
                         // TODO: trap
                     }
                    _ => {
                        panic!("Invalid load funct3code: 0x{:x}", inst.funct3());
                    }
                },
                isa::opcodes::STORE => match inst.funct3() {
                     isa::funct3::SW => {
                        println!("SW");
                    }
                    _ => {
                        panic!("Invalid store funct3code: 0x{:x}", inst.funct3());
                    }
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
}
