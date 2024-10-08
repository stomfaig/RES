#![allow(arithmetic_overflow)]
pub mod cpu {

    use crate::bus::{ControlSignal, Mem};
    use std::{thread, time};

    enum AddressingMode {
        Immediate,
        ZeroPage,
        ZeroPageX,
        ZeroPageY,
        Absolute,
        AbsoluteX,
        AbsoluteY,
        Indirect,
        IndexedIndirectX,
        IndexedIndirectY,
        IndirectIndexedX,
        IndirectIndexedY,
    }

    #[repr(u8)]
    enum Flag {
        N = 0b1000_0000, // negative
        V = 0b0100_0000, // overflow
        //
        B = 0b0001_0000, // B flag
        D = 0b0000_1000, // decimal
        I = 0b0000_0100, // interrupt disable
        Z = 0b0000_0010, // zero
        C = 0b0000_0001, // carry
    }

    pub struct CPU <T: Mem>{
        pub register_a: u8,
        pub register_x: u8,
        pub register_y: u8,
        pub stack_pointer: u8,
        pub status: u8,
        pub program_counter: u16,
        pub debug: bool,
        memory: T,
    }

    // Macro for generating instructions cmp, cpx, cpy
    //
    macro_rules! cp {
        ($($name: ident, $register: ident), +) => {
            $(
                fn $name(&mut self, mode: AddressingMode) {
                    let addr = self.get_target_address(mode);
                    let val = self.mem_read(addr);
                    self.set_flag(Flag::C, self.$register >= val);
                    self.set_flag(Flag::Z, self.$register == val);
                    // need a subtract here...
                }
            )+
        }
    }

    // Macro for generating instructions lda, ldx and ldy.
    // Loads the content of a specified memory address into a specified register.
    macro_rules! ld {
        ($($name: ident, $register: ident),+) => {
            $(
                fn $name(&mut self, mode: AddressingMode) {
                    let addr: u16 = self.get_target_address(mode);
                    self.$register = self.mem_read(addr);
                    self.set_zero(self.$register);
                    self.set_negative(self.$register);
                }
            )+
        }
    }

    // Macro for generating instructions lda, ldx and ldy.
    // Loads the content of a specified memory address into a specified register.
    macro_rules! st {
        ($($name: ident, $register: ident),+) => {
            $(
                fn $name(&mut self, mode: AddressingMode) {
                    let addr: u16 = self.get_target_address(mode);
                    self.mem_write(addr, self.$register);
                }
            )+
        }
    }

    impl<T: Mem> CPU<T> {
        pub fn new(memory: T, debug: bool) -> Self {
            CPU {
                register_a: 0,
                register_x: 0,
                register_y: 0,
                stack_pointer: 0xff,
                status: 0b0010_0000,
                program_counter: 0,
                debug: debug,
                memory: memory,
            }
        }

        fn mem_read(&mut self, addr: u16) -> u8 {
            self.memory.set_control_signal(ControlSignal::MemEnable, false);
            self.memory.set_address_bus(addr);
            self.memory.set_control_signal(ControlSignal::AccessMode, true);
            self.memory.set_control_signal(ControlSignal::MemEnable, true);
            let val: u8 = self.memory.get_data_bus();
            self.memory.set_control_signal(ControlSignal::MemEnable, false);
            val
        }

        fn mem_write(&mut self, addr: u16, value: u8) {
            self.memory.set_control_signal(ControlSignal::MemEnable, false);
            self.memory.set_address_bus(addr);
            self.memory.set_control_signal(ControlSignal::AccessMode, false);
            self.memory.set_data_bus(value);
            self.memory.set_control_signal(ControlSignal::MemEnable, true);
            self.memory.set_control_signal(ControlSignal::MemEnable, false);
        }

        fn mem_read_u16(&mut self, addr: u16) -> u16 {
            let lo = self.mem_read(addr) as u16;
            let hi = self.mem_read(addr + 1) as u16;
            (hi << 8) | lo
        }
        fn mem_write_u16(&mut self, addr: u16, value: u16) {
            let lo = (value & 0xff) as u8;
            let hi = (value >> 8) as u8;
            self.mem_write(addr, lo);
            self.mem_write(addr + 1, hi);
        }

        fn stack_push(&mut self, val: u8) {
            let addr: u16 = 0x0100 + self.stack_pointer as u16;
            self.mem_write(addr, val);
            self.stack_pointer -= 1;
        }

        fn stack_pop(&mut self) -> u8 {
            self.stack_pointer += 1;
            let addr: u16 = 0x0100 + self.stack_pointer as u16;
            let val: u8 = self.mem_read(addr);
            val
        }
        
        fn fetch(&mut self) -> u8 {
            let data = self.mem_read(self.program_counter);
            self.program_counter += 1;
            if self.debug { print!(" {:x}", data) }
            data
        }

        fn set_flag(&mut self, flag: Flag, val: bool) {
            let code = flag as u8;
            if val {
                self.status |= code
            } else {
                self.status &= !code
            };
        }

        fn get_flag(&mut self, flag: Flag) -> bool {
            (self.status & flag as u8) != 0 
        }

        fn get_target_address(&mut self, mode: AddressingMode) -> u16 {
            match mode {
                AddressingMode::Immediate => {self.program_counter += 1; self.program_counter-1},
                AddressingMode::ZeroPage => self.fetch() as u16,
                AddressingMode::ZeroPageX => self.fetch() as u16 + self.register_x as u16,
                AddressingMode::ZeroPageY => self.fetch() as u16 + self.register_y as u16,
                AddressingMode::Absolute => {
                    let lo = self.fetch() as u16;
                    let hi = self.fetch() as u16;
                    hi << 8 | lo
                }
                AddressingMode::AbsoluteX => {
                    let lo = self.fetch() as u16;
                    let hi = self.fetch() as u16;
                    self.register_x as u16 + (hi << 8 | lo)
                }
                AddressingMode::AbsoluteY => {
                    let lo = self.fetch() as u16;
                    let hi = self.fetch() as u16;
                    self.register_y as u16 + (hi << 8 | lo)
                }
                AddressingMode::Indirect => {
                    let val = self.fetch() as u16;
                    self.mem_read_u16(val)
                }
                AddressingMode::IndexedIndirectX => {
                    let val = self.fetch() as u16;
                    self.mem_read_u16(val + self.register_x as u16)
                }
                AddressingMode::IndexedIndirectY => {
                    let val = self.fetch() as u16;
                    self.mem_read_u16(val + self.register_y as u16)
                }
                AddressingMode::IndirectIndexedX => {
                    let val = self.fetch() as u16;
                    self.mem_read_u16(val) + self.register_x as u16
                }
                AddressingMode::IndirectIndexedY => {
                    let val = self.fetch() as u16;
                    self.mem_read_u16(val) + self.register_y as u16
                }
            }
        }

        fn set_zero(&mut self, result: u8) {
            self.set_flag(Flag::Z, result == 0);
        }

        fn set_negative(&mut self, result: u8) {
            let mask: u8 = 0b1000_0000;
            self.set_flag(Flag::N, (mask & result) != 0);
        }

        fn set_carry(&mut self, a: u8, b: u8, result: u8) {
            let mask: u8 = 0b1000_0000;
            self.set_flag(Flag::C, (a | b) & mask != 0 && result & mask == 0);
        }

        fn set_overflow(&mut self, a: u8, b: u8, result: u8) {
            let mask: u8 = 0b1000_0000;
            self.set_flag(
                Flag::V,
                (a & mask == b & mask) && (a & mask != result & mask),
            );
        }

        // adds the contents of a memory location to the accumulator together with the carry bit
        // sets: Carry, Zero, Overflow, Negative
        fn adc(&mut self, mode: AddressingMode) {
            let old: u8 = self.register_a;
            let addr: u16 = self.get_target_address(mode);
            let other: u8 = self.mem_read(addr);
            self.register_a += other;
            self.register_a += self.get_flag(Flag::C) as u8;
            self.set_zero(self.register_a);
            self.set_negative(self.register_a);
            self.set_carry(old, other, self.register_a);
            self.set_overflow(old, other, self.register_a);
        }

        // logical and is performed, bit by bit, on the accumulator contents using the contents of a byte of memory
        // sets: Zero, Negative
        fn and(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            self.register_a &= self.mem_read(addr);
            self.set_zero(self.register_a);
            self.set_negative(self.register_a);
        }

        // shifts all the bits of the accumulator or memory contents one bit left
        // sets: Carry, Zero, Negative
        fn asl(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let old: u8 = self.mem_read(addr);
            let new: u8 = old << 1;
            self.mem_write(addr, new);
            self.set_flag(Flag::C, old & 0b1000_0000 != 0);
            self.set_zero(new);
            self.set_negative(new);
        }

        // This instructions is used to test if one or more bits are set in a target memory location. The mask pattern in A is ANDed with the value in memory to set or clear the zero flag, but the result is not kept. Bits 7 and 6 of the value from memory are copied into the N and V flags.
        // Sets: Zero, Overflow, Carry

        fn bit(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let val: u8 = self.mem_read(addr);
            self.set_flag(Flag::Z, self.register_a & val == 0);
            self.set_flag(Flag::N, val & 0b1000_0000 != 0);
            self.set_flag(Flag::V, val & 0b0100_0000 != 0);
        }

        cp![cmp, register_a, cpx, register_x, cpy, register_y];

        fn jump_rel(&mut self, condition: bool) {
            let rel: u8 = self.fetch();
            if !condition { return; }
            self.program_counter;
            if rel & 0b1000_0000 == 0 {
                self.program_counter += (rel & 0b0111_1111) as u16;
            } else {
                self.program_counter += rel as u16 | 0b1111_1111_0000_0000;
            }
        }

        fn dec(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let val: u8 = self.mem_read(addr) + 0b1111_1111;
            self.mem_write(addr, val);

            self.set_zero(val);
            self.set_negative(val);
        }

        fn eor(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let data: u8 = self.mem_read(addr);
            self.register_a |= data; 
            self.set_zero(self.register_a);
            self.set_negative(self.register_a);
        }

        fn inc(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let val: u8 = self.mem_read(addr) + 0b0000_0001;
            self.mem_write(addr, val);

            self.set_zero(val);
            self.set_negative(val);
        }

        fn jmp(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            //let data: u16 = self.mem_read_u16(addr);
            self.program_counter = addr;
        }

        ld![lda, register_a, ldx, register_x, ldy, register_y];

        fn lsr(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let val: u8 = self.mem_read(addr);
            let new_val: u8 = val >> 1;
            self.mem_write(addr, new_val);

            self.set_flag(Flag::C, val & 0b0000_0001 != 0);
            self.set_zero(new_val);
            self.set_negative(new_val);
        }
        
        fn ora(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let data: u8 = self.mem_read(addr);
            self.register_a |= data;

            self.set_zero(self.register_a);
            self.set_negative(self.register_a);
        }

        /// rol - rotate left
        fn rol(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let val: u8 = self.mem_read(addr);
            let new_val = (val << 1) + self.get_flag(Flag::C) as u8; // maybe need something more intricate here??
            self.mem_write(addr, new_val);
            self.set_flag(Flag::C, val & 0b1000_0000 != 0);
            self.set_zero(new_val);
            self.set_negative(new_val);
        }

        fn ror(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let val: u8 = self.mem_read(addr);
            let new_val = (val >> 1) | ((self.get_flag(Flag::C) as u8) << 7); 
            self.mem_write(addr, new_val);
            self.set_flag(Flag::C, val & 0b0000_0001 != 0);
            self.set_zero(new_val);
            self.set_negative(new_val);
        }

        fn sbc(&mut self, _mode: AddressingMode) {
            todo!();
        }

        st![sta, register_a, stx, register_x, sty, register_y];

        pub fn start(&mut self) {
            //self.program_counter = 0xc000; //
            self.program_counter = self.mem_read_u16(0xFFFC);
            self.run();
        }

        pub fn run(&mut self) {
            loop {
                if self.debug { print!("prg ctr: {:x}, cd:", self.program_counter) }
                let opcode: u8 = self.fetch();

                match opcode {
                    // adc
                    0x69 => self.adc(AddressingMode::Immediate),
                    0x65 => self.adc(AddressingMode::ZeroPage),
                    0x75 => self.adc(AddressingMode::ZeroPageX),
                    0x6d => self.adc(AddressingMode::Absolute),
                    0x7d => self.adc(AddressingMode::AbsoluteX),
                    0x79 => self.adc(AddressingMode::AbsoluteY),
                    0x61 => self.adc(AddressingMode::IndexedIndirectX),
                    0x71 => self.adc(AddressingMode::IndirectIndexedY),
                    // and
                    0x29 => self.and(AddressingMode::Immediate),
                    0x25 => self.and(AddressingMode::ZeroPage),
                    0x35 => self.and(AddressingMode::ZeroPageX),
                    0x2d => self.and(AddressingMode::Absolute),
                    0x3d => self.and(AddressingMode::AbsoluteX),
                    0x39 => self.and(AddressingMode::AbsoluteY),
                    0x21 => self.and(AddressingMode::IndexedIndirectX),
                    0x31 => self.and(AddressingMode::IndirectIndexedY),
                    // asl
                    0x0a => {
                        self.set_flag(Flag::C, self.register_a & 0b1000_0000 != 0);
                        self.register_a = self.register_a << 1;
                        self.set_zero(self.register_a);
                        self.set_negative(self.register_a);
                    },
                    0x06 => self.asl(AddressingMode::ZeroPage),
                    0x16 => self.asl(AddressingMode::ZeroPageX),
                    0x0e => self.asl(AddressingMode::Absolute),
                    0x1e => self.asl(AddressingMode::AbsoluteX),
                    // bcc - Branch if carry clear
                    0x90 => { let carry = self.get_flag(Flag::C); self.jump_rel(!carry); },
                    // bcs - Branch if carry set
                    0xb0 => { let carry = self.get_flag(Flag::C); self.jump_rel(carry); },
                    // beq - Branch if equal
                    0xf0 => { let zero = self.get_flag(Flag::Z); self.jump_rel(zero); },
                    // bit
                    0x24 => self.bit(AddressingMode::ZeroPage),  
                    0x2c => self.bit(AddressingMode::Absolute),
                    // bmi - Branch if minus
                    0x30 => { let neg = self.get_flag(Flag::N); self.jump_rel(neg); },
                    // bne
                    0xd0 => { let zero = self.get_flag(Flag::Z); self.jump_rel(!zero); },
                    // bpl - Branch if positive
                    0x10 => { let neg = self.get_flag(Flag::N); self.jump_rel(!neg); },
                    // brk - force interrupt
                    0x00 => {
                        let lsb: u8 = (self.program_counter & 0xff) as u8;
                        let msb: u8 = (self.program_counter >> 8) as u8;
                        self.stack_push(msb);
                        self.stack_push(lsb);
                        self.stack_push(self.status);
                        
                        self.program_counter = self.mem_read_u16(0xffff);
                        self.set_flag(Flag::B, true);
                    },
                    // bvc - Branch if overflow clear
                    0x50 => { let overflow = self.get_flag(Flag::V); self.jump_rel(!overflow); },
                    // bvs - Branch if overflow set
                    0x70 => { let overflow = self.get_flag(Flag::V); self.jump_rel(overflow); },
                    // clc - Clear carry flag
                    0x18 => self.set_flag(Flag::C, false),
                    // cld - Clear decimal mode
                    0xd8 => self.set_flag(Flag::D, false),
                    // cli - Clear interrupt disable
                    0x58 => self.set_flag(Flag::I, false),
                    // clv - Clear overflow
                    0xb8 => self.set_flag(Flag::V, false),
                    // cmp - compare accumulator with value in memory
                    0xc9 => self.cmp(AddressingMode::Immediate), 
                    0xc5 => self.cmp(AddressingMode::ZeroPage),
                    0xd5 => self.cmp(AddressingMode::ZeroPageX),
                    0xcd => self.cmp(AddressingMode::Absolute),
                    0xdd => self.cmp(AddressingMode::AbsoluteX),
                    0xd9 => self.cmp(AddressingMode::AbsoluteY),
                    0xc1 => self.cmp(AddressingMode::IndexedIndirectX),
                    0xd1 => self.cmp(AddressingMode::IndirectIndexedY),
                    // cpx - compare register x with value in memory
                    0xe0 => self.cpx(AddressingMode::Immediate),
                    0xe4 => self.cpx(AddressingMode::ZeroPage),
                    0xec => self.cpx(AddressingMode::Absolute),
                    // cpy - compare register y with value in memory
                    0xc0 => self.cpy(AddressingMode::Immediate),
                    0xc4 => self.cpy(AddressingMode::ZeroPage),
                    0xcc => self.cpy(AddressingMode::Absolute),
                    // dec - decrement memory
                    0xc6 => self.dec(AddressingMode::ZeroPage),
                    0xd6 => self.dec(AddressingMode::ZeroPageX),
                    0xce => self.dec(AddressingMode::Absolute),
                    0xde => self.dec(AddressingMode::AbsoluteX),
                    // dex - decrease register x
                    0xca => {
                        self.register_x += 0b1111_1111;
                        self.set_zero(self.register_x);
                        self.set_negative(self.register_x);
                    },
                    // dey - decrement register y
                    0x88 => {
                        self.register_y += 0b1111_1111;
                        self.set_zero(self.register_y);
                        self.set_negative(self.register_y);
                    },
                    // eor - exclusive or
                    0x49 => self.eor(AddressingMode::Immediate),
                    0x45 => self.eor(AddressingMode::ZeroPage),
                    0x55 => self.eor(AddressingMode::ZeroPageX),
                    0x4d => self.eor(AddressingMode::Absolute),
                    0x5d => self.eor(AddressingMode::AbsoluteX),
                    0x59 => self.eor(AddressingMode::AbsoluteY),
                    0x41 => self.eor(AddressingMode::IndexedIndirectX),
                    0x51 => self.eor(AddressingMode::IndirectIndexedY),
                    // inc - increment memory
                    0xe6 => self.inc(AddressingMode::ZeroPage),
                    0xf6 => self.inc(AddressingMode::ZeroPageX),
                    0xee => self.inc(AddressingMode::Absolute),
                    0xfe => self.inc(AddressingMode::AbsoluteX),
                    // inx - increment register x
                    0xe8 => {
                        self.register_x += 0b0000_0001;
                        self.set_zero(self.register_x);
                        self.set_negative(self.register_x);
                    },
                    // dey - decrement register y
                    0xc8 => {
                        self.register_y += 0b0000_0001;
                        self.set_zero(self.register_y);
                        self.set_negative(self.register_y);
                    },
                    // jmp - jump
                    0x4c => self.jmp(AddressingMode::Absolute),
                    0x6c => self.jmp(AddressingMode::Indirect),
                    // jsr - jump to subroutine
                    0x20 => {
                        let target_addr: u16 = self.get_target_address(AddressingMode::Absolute);
                        let lsb: u8 = ((self.program_counter) & 0xff) as u8;
                        let msb: u8 = ((self.program_counter) >> 8) as u8;                    
                        self.stack_push(msb);
                        self.stack_push(lsb);
                        self.program_counter = target_addr;
                    }
                    // lda - load accumulator
                    0xa9 => self.lda(AddressingMode::Immediate),
                    0xa5 => self.lda(AddressingMode::ZeroPage),
                    0xb5 => self.lda(AddressingMode::ZeroPageX),
                    0xad => self.lda(AddressingMode::Absolute),
                    0xbd => self.lda(AddressingMode::AbsoluteX),
                    0xb9 => self.lda(AddressingMode::AbsoluteY),
                    0xa1 => self.lda(AddressingMode::IndexedIndirectY),
                    0xb1 => self.lda(AddressingMode::IndirectIndexedY),
                    // ldx - load register x
                    0xa2 => self.ldx(AddressingMode::Immediate),
                    0xa6 => self.ldx(AddressingMode::ZeroPage),
                    0xb6 => self.ldx(AddressingMode::ZeroPageY),
                    0xae => self.ldx(AddressingMode::Absolute),
                    0xbe => self.ldx(AddressingMode::AbsoluteY),
                    // ldy - load register y
                    0xa0 => self.ldy(AddressingMode::Immediate),
                    0xa4 => self.ldy(AddressingMode::ZeroPage),
                    0xb4 => self.ldy(AddressingMode::ZeroPageX),
                    0xac => self.ldy(AddressingMode::Absolute),
                    0xbc => self.ldy(AddressingMode::AbsoluteX),
                    // lsr - logical shift right
                    0x4a => { 
                        self.set_flag(Flag::C, self.register_a & 0b1000_000 != 0);
                        let new_val: u8 = self.register_a >> 1;
                        self.register_a = new_val;
                        self.set_zero(new_val);
                        self.set_negative(new_val);
                    },
                    0x46 => self.lsr(AddressingMode::ZeroPage),
                    0x56 => self.lsr(AddressingMode::ZeroPageX),
                    0x4e => self.lsr(AddressingMode::Absolute),
                    0x54 => self.lsr(AddressingMode::AbsoluteX),
                    // nop - no operation
                    0xea => (),
                    // ora - logical or performed on accumulator
                    0x09 => self.ora(AddressingMode::Immediate),
                    0x05 => self.ora(AddressingMode::ZeroPage),
                    0x15 => self.ora(AddressingMode::ZeroPageX),
                    0x0d => self.ora(AddressingMode::Absolute),
                    0x1d => self.ora(AddressingMode::AbsoluteX),
                    0x19 => self.ora(AddressingMode::AbsoluteY),
                    0x01 => self.ora(AddressingMode::IndexedIndirectX),
                    0x11 => self.ora(AddressingMode::IndirectIndexedY),
                    // pha - push a onto stack
                    0x48 => self.stack_push(self.register_a), 
                    // php - push status onto stack
                    0x08 => self.stack_push(self.status | 0b0001_0000),
                    // pla - pull accumulator
                    0x68 =>  {
                        self.register_a = self.stack_pop();
                        self.set_zero(self.register_a);
                        self.set_negative(self.register_a);
                    },
                    // plp - pull processor status
                    0x28 => self.status = self.stack_pop(),
                    // rol - rotate left
                    0x2a => {
                        let val: u8 = self.register_a;
                        self.register_a = val << 1 + self.get_flag(Flag::C) as u8; // maybe need something more intricate here??
                        self.set_flag(Flag::C, val & 0b1000_0000 != 0);
                        self.set_zero(self.register_a);
                        self.set_negative(self.register_a);
                    },
                    0x26 => self.rol(AddressingMode::ZeroPage),
                    0x36 => self.rol(AddressingMode::ZeroPageX),
                    0x2e => self.rol(AddressingMode::Absolute),
                    0x3e => self.rol(AddressingMode::AbsoluteX),
                    // ror - rotate right
                    0x6a => {
                        let val: u8 = self.register_a;
                        self.register_a = val >> 1 + (0b1000_0000 * (self.get_flag(Flag::C) as u8)); // maybe need something more intricate here??
                        self.set_flag(Flag::C, val & 0b0000_0001 != 0);
                        self.set_zero(self.register_a);
                        self.set_negative(self.register_a);
                    },
                    0x66 => self.ror(AddressingMode::ZeroPage),
                    0x76 => self.ror(AddressingMode::ZeroPageX),
                    0x6e => self.ror(AddressingMode::Absolute),
                    0x7e => self.ror(AddressingMode::AbsoluteX),
                    // rti - return from interrupt
                    0x40 => {
                        self.status = self.stack_pop();
                        let lsb: u8 = self.stack_pop();
                        let msb: u8 = self.stack_pop();
                        self.program_counter = lsb as u16 + (msb as u16) << 8;
                    }
                    // rts - return from subroutine
                    0x60 => {
                        let lsb: u8 = self.stack_pop();
                        let msb: u8 = self.stack_pop();
                        let ret_addr = ((msb as u16) << 8) + (lsb as u16);
                        self.program_counter = ret_addr;
                    }
                    // sbc - subtract with carry
                    0xe9 => self.sbc(AddressingMode::Immediate),
                    0xe5 => self.sbc(AddressingMode::ZeroPage),
                    0xf5 => self.sbc(AddressingMode::ZeroPageX),
                    0xed => self.sbc(AddressingMode::Absolute),
                    0xfd => self.sbc(AddressingMode::AbsoluteX),
                    0xf9 => self.sbc(AddressingMode::AbsoluteY),
                    0xe1 => self.sbc(AddressingMode::IndexedIndirectX),
                    0xf1 => self.sbc(AddressingMode::IndirectIndexedY),
                    // sec - set carry flag
                    0x38 => { self.set_flag(Flag::C, true); },
                    // sed - set decimal flag
                    0xf8 => { self.set_flag(Flag::D, true); },
                    // sei - set interrupt disable 
                    0x78 => { self.set_flag(Flag::I, true); },
                    // sta - store accumulator
                    0x85 => self.sta(AddressingMode::ZeroPage),
                    0x95 => self.sta(AddressingMode::ZeroPageX),
                    0x8d => self.sta(AddressingMode::Absolute),
                    0x9d => self.sta(AddressingMode::AbsoluteX),
                    0x99 => self.sta(AddressingMode::AbsoluteY),
                    0x81 => self.sta(AddressingMode::IndexedIndirectX),
                    0x91 => self.sta(AddressingMode::IndirectIndexedY),
                    // stx - store register x
                    0x86 => self.stx(AddressingMode::ZeroPage),
                    0x96 => self.stx(AddressingMode::ZeroPageY),
                    0x8e => self.stx(AddressingMode::Absolute),
                    // sty - store register y
                    0x84 => self.sty(AddressingMode::ZeroPage),
                    0x94 => self.sty(AddressingMode::ZeroPageX),
                    0x8c => self.sty(AddressingMode::Absolute),
                    // tax - transfer accumulator to x
                    0xaa => {
                        self.register_x = self.register_a;
                        self.set_zero(self.register_x);
                        self.set_negative(self.register_x);
                    },
                    // tay - transfer accumulator to y
                    0xa8 => {
                        self.register_y = self.register_a;
                        self.set_zero(self.register_y);
                        self.set_negative(self.register_y);
                    },
                    // tsx - transfer stack register to x
                    0xba => {
                        self.register_x = self.stack_pointer;
                        self.set_zero(self.register_x);
                        self.set_negative(self.register_x);
                    },
                    // txa - transfer x to accumulator
                    0x8a => {
                        self.register_a = self.register_x;
                        self.set_zero(self.register_a);
                        self.set_negative(self.register_a);
                    },
                    // txs - transfer x to stack pointer
                    0x9a => self.stack_pointer = self.register_x,
                    // tya - transfer y to accumulator
                    0x98 => {
                        self.register_a = self.register_y;
                        self.set_zero(self.register_a);
                        self.set_negative(self.register_a);
                    },
                    _ => panic!("Can't recognize instruction instruction {:?}", opcode),
                }

                let ten_millis = time::Duration::from_millis(100);
                thread::sleep(ten_millis);

                if self.debug {println!("\t\t\tA: {:?} X: {:?}, Y: {:?} \t\t flags: {:#08b}", self.register_a, self.register_x, self.register_y, self.status) }
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use rand::prelude::*;
        
        pub struct TestBus {
            address_bus: u16,
            data_bus: u8,
            control_bus: u8,
            read_targets: HashMap<u16, u8>,
            write_targets: HashMap<u16, u8>,
        }
        
        impl TestBus {
        
            pub fn set_read_target(&mut self, addr: u16, val: u8) {
                self.read_targets.insert(addr, val);
            }
        
            pub fn set_read_u16_target(&mut self, addr: u16, val: u16) {
                let low: u8 = (val & 0xff) as u8;
                let high: u8 = (val >> 8) as u8;
                self.read_targets.insert(addr, low);
                self.read_targets.insert(addr + 1, high);
            }
        
            pub fn set_vector_read_target(&mut self, addr: u16, values: Vec<u8>) {
                let mut offset: u16 = 0;
                for val in values {
                    self.read_targets.insert(addr + offset, val);
                    offset += 1;
                }
            }
        
            pub fn set_write_target(&mut self, addr: u16, val: u8) {
                self.write_targets.insert(addr, val);
            }
        }
        
        impl Mem for TestBus {
            fn new() -> Self {
                Self {
                    address_bus: 0,
                    data_bus: 0,
                    control_bus: 0,
                    read_targets: HashMap::new(),
                    write_targets: HashMap::new(),
                }
            }
            fn set_address_bus(&mut self, addr: u16) {
                self.address_bus = addr;
                
            }
            fn set_data_bus(&mut self, val: u8) {
                self.data_bus = val;
            }
            fn get_data_bus(&self) -> u8 {
                self.data_bus
            }
            fn set_control_signal(&mut self, control: ControlSignal, val: bool) {
                let mask = control as u8;
                if (val)  { self.control_bus |= mask; }
                else { self.control_bus &= !mask; }
                
                if (!self.get_control_signal(ControlSignal::MemEnable)) { return; }
        
                if (self.get_control_signal(ControlSignal::AccessMode)) {
                    let result: Option<&u8> = self.read_targets.get(&self.address_bus);
                    self.data_bus = match result {
                        Some(val) => *val,
                        None => panic!("Method trying to read from forbidden memory (addr: {:x})", self.address_bus),
                    }
                } else {
                    let result: Option<&u8> = self.write_targets.get(&self.address_bus);
                    match result {
                        Some(val) => {
                            if (*val != self.data_bus) { panic!("Method trying to write invalid data(expected: {:b}, got: {:b})", *val, self.data_bus); }
                        },
                        None => panic!("Method trying to write to forbidden memory(addr: {:x}, val: {:b})", self.address_bus, self.data_bus),
                    }
                }
            }
        
            fn get_control_signal(&self, control: ControlSignal) -> bool {
                (self.control_bus & (control as u8)) != 0
            }
        }

        macro_rules! run_test {
            ($instr: ident, $($mode: ident),+) => {
                mod $instr {
                    use super::*;
                    use rand::prelude::*;

                    $(#[test]
                    fn $mode() {
                        let mut cpu = CPU::<TestBus>::new();
                        let mut rng = rand::thread_rng();
                        let mode = AddressingMode::$mode;

                        $instr(&mut cpu, mode, &mut rng);
                    })+
                }
            }
        }
        
        #[inline]
        fn next_u8(rng: &mut ThreadRng) -> u8 {
            (rng.next_u32() & 0xff) as u8
        }

        #[inline]
        fn next_bit(rng: &mut ThreadRng) -> u8 {
            (rng.next_u32() % 2) as u8
        }

        fn adc(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let a = next_u8(rng);
            let c = next_bit(rng);

            let mem_value = next_u8(rng);

            cpu.register_a = a;
            cpu.set_flag(Flag::C, c != 0);
            addressing_mode_tester(cpu, mem_value, &mode);

            cpu.adc(mode);

            assert_eq!(cpu.register_a, a + c + mem_value);
            assert_eq!(cpu.get_flag(Flag::Z), cpu.register_a == 0);
            assert_eq!(cpu.get_flag(Flag::N), cpu.register_a  & 0b1000_0000 != 0);
        }

        run_test![
            adc,
            Immediate,
            ZeroPage,
            ZeroPageX,
            ZeroPageY,
            Absolute,
            AbsoluteX,
            AbsoluteY,
            Indirect,
            IndexedIndirectX,
            IndexedIndirectY,
            IndirectIndexedX,
            IndirectIndexedY
        ];

        fn and(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let a: u8 = next_u8(rng);
            let mem_value: u8 = next_u8(rng);

            cpu.register_a = a;
            addressing_mode_tester(cpu, mem_value, &mode);

            cpu.and(mode);

            assert_eq!(cpu.register_a, a & mem_value);
            assert_eq!(cpu.get_flag(Flag::Z), cpu.register_a == 0);
            assert_eq!(cpu.get_flag(Flag::N), cpu.register_a  & 0b1000_0000 != 0);
        }

        run_test![
            and,
            Immediate,
            ZeroPage,
            ZeroPageX,
            ZeroPageY,
            Absolute,
            AbsoluteX,
            AbsoluteY,
            Indirect,
            IndexedIndirectX,
            IndexedIndirectY,
            IndirectIndexedX,
            IndirectIndexedY
        ];

        fn asl(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let mem_value: u8 = next_u8(rng);
            let addr = addressing_mode_tester(cpu, mem_value, &mode);

            cpu.memory.set_write_target(addr, mem_value << 1);

            cpu.asl(mode);
            
            assert_eq!(cpu.get_flag(Flag::Z), mem_value << 1 == 0);
            assert_eq!(cpu.get_flag(Flag::N), (mem_value << 1)  & 0b1000_0000 != 0);
            assert_eq!(cpu.get_flag(Flag::C), (mem_value) & 0b1000_0000 != 0);
        }

        run_test![
            asl,
            ZeroPage,
            ZeroPageX,
            Absolute,
            AbsoluteX
        ];

        fn bit(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let reg: u8 = next_u8(rng);
            let mem_value: u8 = next_u8(rng);

            cpu.register_a = reg;
            addressing_mode_tester(cpu, mem_value, &mode);

            cpu.bit(mode);
            
            assert_eq!(cpu.get_flag(Flag::Z), reg & mem_value == 0);
            assert_eq!(cpu.get_flag(Flag::N), mem_value & 0b1000_0000 != 0);
            assert_eq!(cpu.get_flag(Flag::V), mem_value & 0b0100_0000 != 0);
        }

        run_test![
            bit,
            ZeroPage,
            Absolute
        ];

        /*  ** Logic check for rel_jump. **
            We simulate that a jump instruction was read at the address 0x8000, and the program counter moved to
            0x8001, where we load the relative jump address. Afterwards, we call the jump_rel instruction logic 
            directly, and check if it set the program counter as expected.
            Note that since the computer is not directly run, we do not need to increase the target program counter
            to deal with the extra 0x00 that is read to halt the execution.
        */
        #[test]
        fn test_rel_jump() {
            let mut cpu = CPU::<TestBus>::new();
            
            cpu.program_counter = 0x8001;
            cpu.memory.set_read_target(0x8001, 0b1001_0101);
            cpu.jump_rel(true);
            assert_eq!(cpu.program_counter, 0x7f95);

            cpu.program_counter = 0x8001;
            cpu.memory.set_read_target(0x8001, 0b0110_0101);
            cpu.jump_rel(true);
            assert_eq!(cpu.program_counter, 0x8065); 
        }

        /*  ** Checking jump instructions **
            In the different test methods we set up the cpu flags according to the instruction tested, and call the jump_check 
            method. This method loads the cpu memory with the instruction tested, and a jump pattern, which allows to test
            if the cpu branched or not. We compare this with the expected behavior.
        */
        fn jump_check(instruction: u8, cpu: &mut CPU<TestBus>) -> bool {
            cpu.program_counter = 0x8000;
            cpu.memory.set_vector_read_target(0x8000, vec![instruction, 0x05, 0x00]);
            cpu.memory.set_read_target(0x8005, 0x00);
            cpu.run();

            match cpu.program_counter {
                0x8006 => true,     // This is 0x8005 + 1, i.e. the program halts on 0x8005
                0x8003 => false,    // Similarly in this case the program halts on 0x8002
                _ => panic!("The value of the program counter is unexpected: {:x}", cpu.program_counter),
            }
        }
        
        #[test]
        fn test_bcc_0x90() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::C, false);
            assert_eq!(jump_check(0x90, &mut cpu), true);

            cpu.set_flag(Flag::C, true);
            assert_eq!(jump_check(0x90, &mut cpu), false);
        }

        #[test]
        fn test_bcs_0xb0() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::C, false);
            assert_eq!(jump_check(0xb0, &mut cpu), false);

            cpu.set_flag(Flag::C, true);
            assert_eq!(jump_check(0xb0, &mut cpu), true);
        }

        #[test]
        fn test_beq_0xf0() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::Z, false);
            assert_eq!(jump_check(0xf0, &mut cpu), false);

            cpu.set_flag(Flag::Z, true);
            assert_eq!(jump_check(0xf0, &mut cpu), true);
        }
        
        #[test]
        fn test_bne_0xd0() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::Z, false);
            assert_eq!(jump_check(0xd0, &mut cpu), true);

            cpu.set_flag(Flag::Z, true);
            assert_eq!(jump_check(0xd0, &mut cpu), false);
        }

        #[test]
        fn test_bmi_0x30() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::N, false);
            assert_eq!(jump_check(0x30, &mut cpu), false);

            cpu.set_flag(Flag::N, true);
            assert_eq!(jump_check(0x30, &mut cpu), true);
        }

        #[test]
        fn test_bpl_0x10() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::N, false);
            assert_eq!(jump_check(0x10, &mut cpu), true);

            cpu.set_flag(Flag::N, true);
            assert_eq!(jump_check(0x10, &mut cpu), false);
        }

        #[test]
        fn test_bvc_0x50() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::V, false);
            assert_eq!(jump_check(0x50, &mut cpu), true);

            cpu.set_flag(Flag::V, true);
            assert_eq!(jump_check(0x50, &mut cpu), false);
        }

        #[test]
        fn test_bvc_0x70() {
            let mut cpu = CPU::<TestBus>::new();

            cpu.set_flag(Flag::V, false);
            assert_eq!(jump_check(0x70, &mut cpu), false);

            cpu.set_flag(Flag::V, true);
            assert_eq!(jump_check(0x70, &mut cpu), true);
        }

        fn dec(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let mem_value: u8 = next_u8(rng);
            let addr = addressing_mode_tester(cpu, mem_value, &mode);
            let new_value = mem_value - 1;

            cpu.memory.set_write_target(addr, new_value);

            cpu.dec(mode);

            assert_eq!(cpu.get_flag(Flag::Z), new_value == 0);
            assert_eq!(cpu.get_flag(Flag::N), (new_value) & 0b1000_0000 != 0);
        }

        run_test![
            dec,
            ZeroPage,
            ZeroPageX,
            Absolute,
            AbsoluteX
        ];

        // what does inc do? well, it increments a memory address...
        fn inc(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let val = next_u8(rng);
            let addr = addressing_mode_tester(cpu, val, &mode);
            cpu.memory.set_write_target(addr, val+1);

            cpu.inc(mode);

            assert_eq!(cpu.get_flag(Flag::Z), val+1 == 0);
            assert_eq!(cpu.get_flag(Flag::N), ((val + 1) & 0b1000_0000) != 0);
        }

        run_test![inc, ZeroPage, ZeroPageX, Absolute, AbsoluteX];

        macro_rules! ld {
            ($($name: ident, $register: ident),+) => {
                $(fn $name(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
                    let val = next_u8(rng);
                    addressing_mode_tester(cpu, val, &mode);
                    cpu.$name(mode);

                    assert_eq!(cpu.$register, val);
                })+

            }
        }

        ld![lda, register_a, ldx, register_x, ldy, register_y];
        run_test![lda, Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndexedIndirectY, IndirectIndexedY];
        run_test![ldx, Immediate,ZeroPage,ZeroPageY,Absolute,AbsoluteY];
        run_test![ldy, Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX];

        fn lsr(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let val = next_u8(rng);
            let addr: u16 = addressing_mode_tester(cpu, val, &mode);
            let new_val = val >> 1;
            cpu.memory.set_write_target(addr, new_val);

            cpu.lsr(mode);

            assert_eq!(cpu.get_flag(Flag::C), val & 0b0000_0001 != 0);
            assert_eq!(cpu.get_flag(Flag::Z), new_val == 0);
            assert_eq!(cpu.get_flag(Flag::N), new_val & 0b1000_0000 != 0);
        }

        run_test![lsr, ZeroPage, ZeroPageX, Absolute, AbsoluteX];

        fn ora(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let val: u8 = next_u8(rng);
            addressing_mode_tester(cpu, val, &mode);
            let reg = next_u8(rng);
            cpu.register_a = reg;

            cpu.ora(mode);

            assert_eq!(cpu.register_a, val | reg);
            assert_eq!(cpu.get_flag(Flag::Z), cpu.register_a == 0);
            assert_eq!(cpu.get_flag(Flag::N), cpu.register_a & 0b1000_0000 != 0);
        }

        run_test![ora, Immediate, ZeroPage, ZeroPageX, Absolute, AbsoluteX, AbsoluteY, IndexedIndirectX, IndirectIndexedY];

        // push instructions

        // rti . return from interrupt

        // rts - return from subroutine

        fn rol(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let carry = next_bit(rng);
            cpu.set_flag(Flag::C, carry != 0);
            let val: u8 = next_u8(rng);
            let addr: u16 = addressing_mode_tester(cpu, val, &mode);

            let target_val = (val << 1) + carry;
            cpu.memory.set_write_target(addr, target_val);

            cpu.rol(mode);
            
            assert_eq!(cpu.get_flag(Flag::C), val & 0b1000_0000 != 0);
            assert_eq!(cpu.get_flag(Flag::Z), target_val == 0);
            assert_eq!(cpu.get_flag(Flag::N), target_val & 0b1000_0000 != 0);
        }

        run_test![rol, ZeroPage, ZeroPageX, Absolute, AbsoluteX];
        
        fn ror(cpu: &mut CPU<TestBus>, mode: AddressingMode, rng: &mut ThreadRng) {
            let carry = next_bit(rng);
            cpu.set_flag(Flag::C, carry != 0);
            let val: u8 = next_u8(rng);
            let addr: u16 = addressing_mode_tester(cpu, val, &mode);

            let target_val = (val >> 1) | (carry << 7);
            cpu.memory.set_write_target(addr, target_val);

            cpu.ror(mode);
            
            assert_eq!(cpu.get_flag(Flag::C), val & 0b0000_0001 != 0);
            assert_eq!(cpu.get_flag(Flag::Z), target_val == 0);
            assert_eq!(cpu.get_flag(Flag::N), target_val & 0b1000_0000 != 0);
        }

        run_test![ror, ZeroPage, ZeroPageX, Absolute, AbsoluteX];

        // Given a cpu and an addressing mode, this method plants a random number in a pre-defined location according to the indexing procedure, and generates code to to access the hidden information.
        fn addressing_mode_tester(cpu: &mut CPU<TestBus>, secret_value: u8, mode: &AddressingMode) -> u16 {
            let lsb: u8 = 10;
            let msb: u8 = 13;
            let addr: u16 = (msb as u16) << 8 + (lsb as u16);
            let reg: u8 = 53;
            let indirect: u16 = 745;

            cpu.program_counter = 0;

            match mode {
                AddressingMode::Immediate => { 
                    cpu.memory.set_read_target(cpu.program_counter, secret_value);
                    cpu.program_counter
                }
                AddressingMode::ZeroPage => {
                    cpu.memory.set_read_target(lsb as u16, secret_value);
                    cpu.memory.set_read_target(cpu.program_counter, lsb);
                    lsb as u16
                }
                AddressingMode::ZeroPageX => {
                    cpu.register_x = reg;
                    cpu.memory.set_read_target(lsb as u16 + reg as u16, secret_value);
                    cpu.memory.set_read_target(cpu.program_counter, lsb);
                    lsb as u16 + reg as u16
                }
                AddressingMode::ZeroPageY => {
                    cpu.register_y = reg;
                    cpu.memory.set_read_target(lsb as u16 + reg as u16, secret_value);
                    cpu.memory.set_read_target(cpu.program_counter, lsb);
                    lsb as u16
                }
                AddressingMode::Absolute => {
                    cpu.memory.set_read_target(addr, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    addr
                }
                AddressingMode::AbsoluteX => {
                    cpu.register_x = reg;
                    cpu.memory.set_read_target(addr + reg as u16, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::AbsoluteY => {
                    cpu.register_y = reg;
                    cpu.memory.set_read_target(addr + reg as u16, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::Indirect => {
                    cpu.memory.set_read_u16_target(addr, indirect);
                    cpu.memory.set_read_target(indirect, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    indirect
                }
                AddressingMode::IndexedIndirectX => {
                    cpu.register_x = reg;
                    cpu.memory.set_read_u16_target(addr + reg as u16, indirect);
                    cpu.memory.set_read_target(indirect, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::IndexedIndirectY => {
                    cpu.register_y = reg;
                    cpu.memory.set_read_u16_target(addr + reg as u16, indirect);
                    cpu.memory.set_read_target(indirect, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::IndirectIndexedX => {
                    cpu.register_x = reg;
                    cpu.memory.set_read_u16_target(addr, indirect);
                    cpu.memory.set_read_target(indirect + reg as u16, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    indirect + (reg as u16)
                }
                AddressingMode::IndirectIndexedY => {
                    cpu.register_y = reg;
                    cpu.memory.set_read_u16_target(addr, indirect);
                    cpu.memory.set_read_target(indirect + reg as u16, secret_value);
                    cpu.memory.set_read_u16_target(cpu.program_counter, addr);
                    indirect + (reg as u16)
                }
            }
        }
    }
}