use isa;
use binary::{Binary};
use memory::{Memory};

pub struct Simulator {
    num_cores: usize,
    memory: Memory,
}

#[derive(Clone)]
struct Core {
    pc: usize,
}

struct RegisterFile {
    registers: [u32; 32],
}

impl RegisterFile {
    fn write_word<T: Into<isa::Register>>(&mut self, reg: T, value: u32) {
        // TODO: should be safe to use unchecked index
        self.registers[reg.into().as_num()] = value;
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
        let mut cores = vec![Core { pc: 0x10000, }; self.num_cores];
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
                        println!("ADDI");
                    }
                    _ => {
                        panic!("Invalid integer-immediate funct3code: 0x{:x}", inst.funct3());
                    }
                },
                isa::opcodes::LOAD => match inst.funct3() {
                     isa::funct3::LW => {
                        println!("LW");
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
