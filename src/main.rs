#![allow(non_snake_case)]
extern crate minifb;

use minifb::{Key, Window, WindowOptions};
use std::fs::File;
use std::io::Read;
use std::process;

struct CPU {
    // Chip 8 has 35 opcodes
    // Each are 2 bytes long
    opcode: u16,

    // Chip 8 has 4K memory
    memory: [u16; 4096],

    // Graphics buffer
    height: u32,
    width: u32,
    gfx: Vec<u32>,

    // CPU Registers
    V: [u8; 16],

    // Index Registers:
    // I and Program Counter
    I: u16,
    pc: u16,
    k: u8,

    // Maintains current location
    // before jumps are performed
    stack: [u16; 16],
    sp: u8,
}

impl CPU {
    fn initialize(path: &str, gfx: Vec<u32>) -> CPU {
        // Loading game file into buffer
        let mut f = File::open(path).unwrap();
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer);

        // Initializing and loading memory
        let mut memory: [u16; 4096] = [0x0000; 4096];

        for (i, _) in buffer.iter().enumerate() {
            // println!("pos {}: {:#06x}", i, buffer[i]);
            memory[i + 512] = buffer[i].into();
        }

        let chip8_fontset = vec![
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];

        let start_loc = 0x50;
        for (index, font) in chip8_fontset.iter().enumerate() {
            memory[start_loc + index] = *font as u16;
        }

        // for (i, buf) in memory[..511].iter().enumerate() {
        // println!("pos {}: {:#06x}", i, buf)
        // }

        CPU {
            opcode: 0,
            memory: memory,
            height: 32,
            width: 64,
            gfx: gfx,
            V: [0x0000; 16],
            I: 0,
            pc: 0x200,
            stack: [0x0000; 16],
            sp: 0,
            k: 0,
        }
    }

    fn emulate_cycle(&mut self) {
        // Opcodes are stored in two memory locations
        // We need both to get the full opcode
        let opcode_pt_1 = self.pc as usize;
        let opcode_pt_2 = (self.pc + 1) as usize;

        self.opcode = self.memory[opcode_pt_1] << 8 | self.memory[opcode_pt_2];

        let decode = self.opcode & 0xF000;

        CPU::debug_opcode(self.opcode, decode);
        println!("Pt1: {}, Pt2: {}", opcode_pt_1, opcode_pt_2);
        println!(
            "Memory Loc 1: {:#06x}, Memory Loc 2: {:#06x}",
            self.memory[opcode_pt_1], self.memory[opcode_pt_2]
        );

        match decode {
            0x0000 => match self.opcode & 0x00FF {
                // 00E0: Clears the screen
                0x00E0 => {
                    for i in self.gfx.iter_mut() {
                        *i = 0;
                    }

                    self.pc += 2;
                }
                // 00EE Returns from a subroutine
                0x0EE => {
                    self.sp -= 1;
                    self.pc = self.stack[self.sp as usize] as u16;
                    println!("Subroutine Return: {}", self.stack[self.sp as usize]);
                    println!("Stack: {:?} Stack Pos: {}", self.stack, self.sp);
                    self.pc += 2;
                }
                // 0NNN: Jump to machine code routine - Interpreter will ignore
                _ => {
                    // TODO Jump to machine code routine
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
            },
            // 1NNN: Jumps to address NNN.
            0x1000 => {
                let jump_loc = self.opcode & 0x0FFF;
                self.pc = jump_loc;
            }
            // 2NNN: Calls subroutine at NNN
            0x2000 => {
                let jump_loc = self.opcode & 0x0FFF;

                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = jump_loc;
                println!("Subroutine Jump: {}", jump_loc);
                println!("Stack: {:?} Stack Pos: {}", self.stack, self.sp);
            }
            // 3XNN: Skip next instruction if VX equals NN
            0x3000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                if self.V[VX] == (self.opcode & 0x00FF) as u8 {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            // 4XNN: Skip next instruction if VX does not equal NN
            0x4000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                if self.V[VX] != (self.opcode & 0x00FF) as u8 {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            // 6XNN: Sets VX to NN
            0x6000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                let NN = self.opcode & 0x00FF;
                self.V[VX] = NN as u8;
                self.pc += 2;
            }
            // 7XNN: Adds NN to VX
            0x7000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                let NN = (self.opcode & 0x00FF) as u8;
                self.V[VX] = (self.V[VX] + NN) % std::u8::MAX;
                self.pc += 2;
            }
            0x8000 => match self.opcode & 0x000F {
                // Sets VX to value of VY
                0x0000 => {
                    let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                    let VY = ((self.opcode & 0x00F0) >> 4) as usize;
                    let VY = self.V[VY];
                    self.V[VX] = VY;
                    self.pc += 2;
                }
                0x0001 => {
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
                0x0002 => {
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
                // 8XY3: Sets VX to VX xor VY
                0x0003 => {
                    let VX = (self.opcode & 0x0F00) >> 8;
                    let VY = (self.opcode & 0x00F0) >> 4;
                    self.V[VX as usize] = (VX ^ VY) as u8;
                    self.pc += 2;
                }
                0x0004 => {
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
                0x0005 => {
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
                0x0006 => {
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
                0x0007 => {
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
                _ => {
                    println!("0x8XYN Undetermined Opcode!");
                    CPU::debug_opcode(self.opcode, decode);
                    process::exit(0x0100);
                }
            },
            // ANNN: Set I to address at NNN
            0xA000 => {
                println!("{}", self.I);
                self.I = (self.opcode & 0x0FFF);
                println!("{}", self.I);
                self.pc += 2;
            }
            // DXYN: Draw at (Vx, Vy, N)
            0xD000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                let VY = ((self.opcode & 0x00F0) >> 4) as usize;
                let n: u16 = (self.opcode & 0x000F) as u16; // Height of gfx

                let x: u16 = self.V[VX] as u16;
                let y: u16 = self.V[VY] as u16;
                let w = (self.width - 1) as u16;

                self.V[0xF] = 0;
                for i in 0..n {
                    let pixel = self.memory[(self.I + i) as usize];
                    println!("{:#08b}", pixel);
                    for j in 0..8 {
                        if pixel & (0x80 >> j) != 0 {
                            let loc = x + j + ((y + i) * 64);
                            if self.gfx[loc as usize] == 1 {
                                self.V[0xF] = 1;
                            }
                            self.gfx[loc as usize] ^= 1;
                        }
                    }
                }
                let mut line = String::new();
                let _ = std::io::stdin().read_line(&mut line).unwrap();

                self.pc += 2;
            }
            0xE000 => {
                match self.opcode & 0x00FF {
                    // EXA1: Skips the next instruction if the key stored in VX isn't
                    // pressed. (Usually the next instruction is a jump to skip a code block)
                    0x00a1 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as u8;
                        if VX == self.k {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    _ => {
                        println!("Undetermined Opcode!");
                        CPU::debug_opcode(self.opcode, decode);
                        process::exit(0x0100);
                    }
                }
            }
            // FNNN: Opcodes for F parsed here
            0xF000 => {
                match self.opcode & 0x00FF {
                    //FX1e: Adds VX to I. VF is not affected
                    0x001e => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        let inc = self.V[VX];
                        self.I += inc as u16;
                        self.pc += 2;
                    }
                    0x0015 => {
                        println!("Implement delay timer");
                        self.pc += 2;
                    }
                    // FX18: Sets the sound timer to VX
                    0x0018 => {
                        println!("Implement sound timer");
                        self.pc += 2;
                    }
                    // FX29: Sets I to the location of sprite in VX
                    0x0029 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        println!("VX: {}", VX);
                        println!("V: {:?}", self.V);

                        self.I = 0x50 + self.V[VX] as u16;
                        println!("I: {}", self.I);
                        println!("memory[{}]: {:?}", self.I, self.memory[self.I as usize]);
                        self.pc += 2;
                    }
                    // FX33: Store binary-coded decimal values in memory
                    // Hundreds digit in memory location I
                    // Tens digit in memory location I+1
                    // Ones digit in memory location I+2
                    0x0033 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.memory[self.I as usize] = (self.V[VX] / 100) as u16;
                        self.memory[(self.I + 1) as usize] = ((self.V[VX] / 10) % 10) as u16;
                        self.memory[(self.I + 2) as usize] = ((self.V[VX] % 100) % 10) as u16;

                        self.pc += 2;
                    }
                    // FX55: Stores V0 to VX (including VX) in memory starting at
                    // address I. The offset from I is increased by 1 for each value
                    // written, but I itself is left unmodified.[d]
                    0x0055 => {
                        let VX = (self.opcode & 0x0F00) >> 8;
                        println!("VX: {}", VX);
                        println!("V: {:?}", self.V);

                        for x in 0..VX + 1 {
                            let V_index = x as usize;
                            let memory_index = (self.I + x) as usize;
                            self.memory[memory_index] = self.V[V_index] as u16;
                        }

                        self.pc += 2;
                    }
                    // FX65: Fill V0 to VX with values starting from memory I
                    // I is increased by 1 each cycle, but is left unmodified
                    0x0065 => {
                        let VX = (self.opcode & 0x0F00) >> 8;
                        println!("VX: {}", VX);
                        println!("V: {:?}", self.V);

                        for x in 0..VX + 1 {
                            let V_index = x as usize;
                            let memory_index = (self.I + x) as usize;
                            self.V[V_index] = self.memory[memory_index] as u8;
                        }

                        self.pc += 2;
                    }
                    _ => {
                        println!("2) Undetermined Opcode!");
                        CPU::debug_opcode(self.opcode, decode);
                        process::exit(0x0100);
                    }
                }
            }
            // Exit and print last opcode
            _ => {
                println!("1) Undetermined Opcode!");
                CPU::debug_opcode(self.opcode, decode);
                process::exit(0x0100);
            }
        }
    }

    fn debug_opcode(opcode: u16, decode: u16) {
        println!("\nOpcode: {:#06x}", opcode);
        println!("Decode: {:#06x}", decode)
    }

    fn debug_cpu_registers(V: [u16; 16]) {
        for (i, code) in V.iter().enumerate() {
            println!("V[{}]: {:#06x}", i, code);
        }
    }
}

fn main() {
    let height: usize = 32;
    let width: usize = 64;

    let gfx: Vec<u32> = vec![0; width * height];

    //let path = "pong.ch8";
    let path = "c8games/UFO";
    let mut cpu = CPU::initialize(path, gfx);

    let mut window = Window::new(
        "Chip-8 - Press ESC to exit",
        width,
        height,
        WindowOptions::default(),
    )
    .expect("Unable to create window");

    // Limit to max ~60 fps update rate
    //window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
    let pixel_color_white = std::u32::MAX;
    let pixel_color_black = 0;
    let mut buffer: Vec<u32> = vec![0; width * height];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        cpu.emulate_cycle();

        for (index, i) in cpu.gfx.iter_mut().enumerate() {
            let mut color = pixel_color_black;
            if *i == 1 {
                color = pixel_color_white;
            }
            buffer[index] = color;
            //*i = std::u32::MAX; // write something more funny here!
        }
        //println!("{:?}", buffer);

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, width, height).unwrap();
    }
    process::exit(0x0100);
}
