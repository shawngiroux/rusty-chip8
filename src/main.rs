#![allow(non_snake_case)]
extern crate minifb;

use minifb::{Key, Window, WindowOptions};
use rand::thread_rng;
use rand::Rng;
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

    delay_timer: u8,
    sound_timer: u8,
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
            delay_timer: 0,
            sound_timer: 0,
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
                let NN = (self.opcode & 0x00FF);
                self.V[VX] = (self.V[VX] as u16 + NN) as u8;
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
                // 8XY1: Sets VX to VX or VY. (Bitwise OR operation)
                0x0001 => {
                    let VX = (self.opcode & 0x0F00) >> 8;
                    let VY = (self.opcode & 0x00F0) >> 4;
                    self.V[VX as usize] = (VX | VY) as u8;
                    self.pc += 2;
                }
                // 8XY2: Sets VX to VX and VY. (Bitwise AND operation)
                0x0002 => {
                    let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                    let VY = ((self.opcode & 0x00F0) >> 4) as usize;
                    self.V[VX] = (self.V[VX] & self.V[VY]) as u8;
                    self.pc += 2;
                }
                // 8XY3: Sets VX to VX xor VY
                0x0003 => {
                    let VX = (self.opcode & 0x0F00) >> 8;
                    let VY = (self.opcode & 0x00F0) >> 4;
                    self.V[VX as usize] = (VX ^ VY) as u8;
                    self.pc += 2;
                }
                // 8XY4: Adds VY to VX. VF is set to 1 when there's a carry,
                // and to 0 when there isn't.
                0x0004 => {
                    let VX = (self.opcode & 0x0F00) >> 8;
                    let VY = (self.opcode & 0x00F0) >> 4;

                    let add = self.V[VX as usize] as u16 + self.V[VY as usize] as u16;

                    self.V[0xf] = if add > 255 { 1 } else { 0 };

                    self.V[VX as usize] = add as u8;

                    self.pc += 2;
                }
                // 8XY5: VY is subtracted from VX. VF is set to 0 when there's
                // a borrow, and 1 when there isn't.
                0x0005 => {
                    let VX = (self.opcode & 0x0F00) >> 8;
                    let VY = (self.opcode & 0x00F0) >> 4;

                    let sub = ((self.V[VX as usize] as i16) - (self.V[VY as usize] as i16));

                    self.V[0xf] = if sub < 0 { 0 } else { 1 };

                    self.V[VX as usize] = sub as u8;

                    self.pc += 2;
                }
                // 8XY6: Stores the least significant bit of VX in VF and then
                // shifts VX to the right by 1.
                0x0006 => {
                    let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                    let VY = ((self.opcode & 0x00F0) >> 4) as usize;

                    println!("Implement least significant bit into VF");

                    println!("V[X]: {:#06x}", self.V[VX]);
                    println!("V[Y]: {:#06x}", self.V[VY]);

                    self.V[VX] = self.V[VY] << 1;

                    self.pc += 2;
                }
                // 8XY7: Sets VX to VY minus VX. VF is set to 0 when there's a
                // borrow, and 1 when there isn't.
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
            // 9XY0: Skips the next instruction if VX doesn't equal VY.
            // (Usually the next instruction is a jump to skip a code block)
            0x9000 => {
                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                let VY = ((self.opcode & 0x00F0) >> 4) as usize;

                if self.V[VX] != self.V[VY] {
                    self.pc += 4
                } else {
                    self.pc += 2
                }
            }
            // ANNN: Set I to address at NNN
            0xA000 => {
                self.I = (self.opcode & 0x0FFF);
                self.pc += 2;
            }
            // CXNN: Sets VX to the result of a bitwise and operation on a
            // random number (Typically: 0 to 255) and NN.
            0xC000 => {
                let mut rng = thread_rng();
                let num: u16 = rng.gen_range(0, 255);

                let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                let NN = (self.opcode & 0x00FF);

                self.V[VX] = (num & NN) as u8;

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
                //let mut line = String::new();
                //let _ = std::io::stdin().read_line(&mut line).unwrap();
                //process::exit(0x0100);

                self.pc += 2;
            }
            0xE000 => {
                match self.opcode & 0x00FF {
                    // EX9E: Skips the next instruction if the key stored in VX
                    // is pressed. (Usually the next instruction is a jump to
                    // skip a code block)
                    0x009e => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as u8;
                        if VX == self.k {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    // EXA1: Skips the next instruction if the key stored in VX
                    // isn't pressed. (Usually the next instruction is a jump
                    // to skip a code block)
                    0x00a1 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as u8;
                        if VX != self.k {
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
                    // FX0A: A key press is awaited, and then stored in VX.
                    // (Blocking Operation. All instruction halted until next
                    // key event)
                    0x000A => {
                        // TODO Halt until key press
                        while self.k == 0xff {
                            println!("Waiting for key press");
                        }
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.V[VX] = self.k;
                        self.pc += 2;
                    }
                    //FX1e: Adds VX to I. VF is not affected
                    0x001e => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        let inc = self.V[VX];
                        self.I += inc as u16;
                        self.pc += 2;
                    }
                    // FX07: Store the current value of the delay timer in
                    // register VX
                    0x0007 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.V[VX] = self.delay_timer;
                        self.pc += 2;
                    }
                    // FX15: Set the delay timer to the value of register VX
                    0x0015 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.delay_timer = self.V[VX];
                        self.pc += 2;
                    }
                    // FX18: Sets the sound timer to VX
                    0x0018 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.sound_timer = self.V[VX];
                        self.pc += 2;
                    }
                    // FX29: Sets I to the location of sprite in VX
                    0x0029 => {
                        let VX = ((self.opcode & 0x0F00) >> 8) as usize;
                        self.I = 0x50 + self.V[VX] as u16;
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
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
    let pixel_color_white = std::u32::MAX;
    let pixel_color_black = 0;
    let mut buffer: Vec<u32> = vec![0; width * height];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.get_keys().map(|keys| {
            for t in keys {
                match t {
                    Key::Key1 => cpu.k = 0x0,
                    Key::Key2 => cpu.k = 0x1,
                    Key::Key3 => cpu.k = 0x2,
                    Key::Key4 => cpu.k = 0x3,
                    Key::Q => cpu.k = 0x4,
                    Key::W => cpu.k = 0x5,
                    Key::E => cpu.k = 0x6,
                    Key::R => cpu.k = 0x7,
                    Key::A => cpu.k = 0x8,
                    Key::S => cpu.k = 0x9,
                    Key::D => cpu.k = 0xa,
                    Key::F => cpu.k = 0xb,
                    Key::Z => cpu.k = 0xc,
                    Key::X => cpu.k = 0xd,
                    Key::C => cpu.k = 0xe,
                    Key::V => cpu.k = 0xf,
                    _ => cpu.k = 0xff,
                }
            }
        });

        println!("Current Key Register: {}", cpu.k);
        cpu.emulate_cycle();

        for (index, i) in cpu.gfx.iter_mut().enumerate() {
            let mut color = pixel_color_black;
            if *i == 1 {
                color = pixel_color_white;
            }
            buffer[index] = color;
        }

        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, width, height).unwrap();

        if cpu.delay_timer > 0 {
            cpu.delay_timer -= 1;
        }

        if cpu.sound_timer > 0 {
            cpu.sound_timer -= 1;
            if cpu.sound_timer == 1 {
                println!("BEEP!");
            }
        }

        cpu.k = 0xff; // Reset key press
    }
    process::exit(0x0100);
}
