use isa;
use binary::{Binary};
use memory::{Memory};

pub struct Simulator {
    binary: Binary,
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

impl Simulator {
    pub fn new(num_cores: usize, binary: Binary) -> Simulator {
        Simulator {
            binary: binary,
            num_cores: num_cores,
            memory: Memory::new(0x20000),
        }
    }

    pub fn run(&mut self) {
        let mut cores = vec![Core { pc: 0x10000, }; self.num_cores];
        // TODO: set up memory, cache, devices
        // TODO: map binary into RAM
        loop {
            for core in cores.iter_mut() {
                self.step_core(core);
            }
        }
    }

    fn step_core(&mut self, core: &mut Core) {
        if let Some(inst) = self.memory.read_instruction(core.pc) {
            match inst.opcode() {
                isa::opcodes::Branch => {
                    
                }
                isa::opcodes::IntegerImmediate => {
                    
                }
                _ => {
                    
                }
            }
        }
        else {
            // trap
        }
    }
}
