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
            // 2NNN: Calls subroutine at NNN
            0x2000 => {
                // TODO call subroutine
                let jump_loc = self.opcode & 0x0FFF;

                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = jump_loc;
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
            // 6XNN: Sets VX to NN
            0x6000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                let NN = self.opcode & 0x00FF;
                self.V[VX] = NN;
                self.pc += 2;
            },
            // ANNN: Set I to address at NNN
            0xA000 => {
                self.I = self.opcode & 0x0FFF;
                self.pc += 2;
            },
            // DXYN: Draw at (Vx, Vy, N)
            0xD000 => {
                // TODO draw sprite
                self.pc += 2;
            },
            // FNNN: Opcodes for F parsed here
            0xF000 => {
                match self.opcode & 0x00FF {
                    // FX29: Sets I to the location of sprite in VX
                    0x0029 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.I = self.V[VX];
                        self.pc += 2;
                    },
                    // FX33: Store binary-coded decimal values in memory
                    // Hundreds digit in memory location I
                    // Tens digit in memory location I+1
                    // Ones digit in memory location I+2
                    0x0033 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        let padded_decimal = format!("{:03}", self.V[VX]);

                        // Iterate through binary-coded decimal values
                        let mut count = 0;
                        for bcd in padded_decimal.chars() {
                            let memory_index = (self.I + count) as usize;
                            self.memory[memory_index] = bcd as u16;
                            count += 1;

                        }

                        self.pc += 2;
                    },
                    // FX65: Fill V0 to VX with values starting from memory I
                    // I is increased by 1 each cycle, but is left unmodified
                    0x0065 => {
                        let VX = (self.opcode & 0x0F00) >> 8;

                        for x in 0..VX + 1 {
                            let V_index = x as usize;
                            let old_V = self.V[V_index];
                            let memory_index = (self.I + x) as usize;
                            self.V[x as usize] = self.memory[memory_index];
                        }

                        self.pc += 2;
                    },
                    _ => {
                        println!("Undetermined Opcode!");
                        CPU::debug_opcode(self.opcode, decode);
                        process::exit(0x0100);
                    }
                }
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
    let path = "pong.ch8";
    let mut cpu = CPU::initialize(path);
    loop {
        cpu.emulate_cycle();
    }
}
