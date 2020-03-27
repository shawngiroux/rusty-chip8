use std::io::Read;
use std::fs::File;
use std::process;

struct CPU {
    // Chip 8 has 35 opcodes
    // Each are 2 bytes long
    opcode: u16,

    // Chip 8 has 4K memory
    memory: [u16; 4096],

    // CPU Registers
    V: [u16; 16],

    // Index Registers:
    // I and Program Counter
    I: u16,
    pc: u16,

    // Maintains current location
    // before jumps are performed
    stack: [u16; 16],
    sp: u16,
}

impl CPU {
    fn initialize(path: &str) -> CPU {
        // Loading game file into buffer
        let mut f = File::open(path).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer);

        // Initializing and loading memory
        let mut memory: [u16; 4096] = [0x0000; 4096];

        for (i, _) in buffer.iter().enumerate() {
            // println!("pos {}: {:#06x}", i, buf)
            memory[i + 512] = buffer[i].into();
        }

        // for (i, buf) in memory.iter().enumerate() {
        //     println!("pos {}: {:#06x}", i, buf)
        // }

        CPU {
            opcode: 0,
            memory: memory,
            V: [0x0000; 16],
            I: 0,
            pc: 0x200,
            stack: [0x0000; 16],
            sp: 0
        }
    }

    fn emulate_cycle(&mut self) {
        // Opcodes are stored in two memory locations
        // We need both to get the full opcode
        let opcode_pt_1 = self.pc as usize;
        let opcode_pt_2 = (self.pc + 1) as usize;

        self.opcode = self.memory[opcode_pt_1] << 8 | self.memory[opcode_pt_2];

        let decode = self.opcode & 0xF000;
        match decode {
            // 0NNN: Jump to machine code routine - Interpreter will ignore
            0x0000 => {
                self.pc += 2;
            },
            // 3XNN: Skip next instruction if VX equals NN
            0x3000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                if self.V[VX] == self.opcode & 0x00FF {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            },
            // ANNN: Set I to address at NNN
            0xA000 => {
                self.I = self.opcode & 0x0FFF;
                self.pc += 2;
            },
            // Exit and print last opcode
            _ => {
                println!("Undetermined Opcode!");
                CPU::debug_opcode(self.opcode, decode);
                process::exit(0x0100);
            }
        }
    }

    fn debug_opcode(opcode: u16, decode: u16) {
        println!("Opcode: {:#06x}", opcode);
        println!("Decode: {:#06x}", decode)
    }
}

fn main() {
    let path = "/home/calico/Programming/rust/rusty-chip8/pong.ch8";
    let mut cpu = CPU::initialize(path);
    loop {
        cpu.emulate_cycle();
    }
}
