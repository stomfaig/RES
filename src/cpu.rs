#![allow(arithmetic_overflow)]
mod cpu {
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
        C = 0b0100_0000,
        Z = 0b0010_0000,
        I = 0b0001_0000,
        D = 0b0000_1000,
        B = 0b0000_0100,
        V = 0b0000_0010,
        N = 0b0000_0001,
    }

    pub struct CPU {
        pub register_a: u8,
        pub register_x: u8,
        pub register_y: u8,
        pub status: u8,
        pub program_counter: u16,
        memory: [u8; 0xffff],
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

    impl CPU {
        pub fn new() -> Self {
            CPU {
                register_a: 0,
                register_x: 0,
                register_y: 0,
                status: 0,
                program_counter: 0,
                memory: [0; 0xffff],
            }
        }

        fn mem_read(&self, addr: u16) -> u8 {
            self.memory[addr as usize]
        }

        fn mem_write(&mut self, addr: u16, value: u8) {
            self.memory[addr as usize] = value;
        }

        fn mem_read_u16(&self, addr: u16) -> u16 {
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

        pub fn load_and_run(&mut self, program: Vec<u8>, reset: bool) {
            self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
            self.program_counter = self.mem_read_u16(0xfffc);

            if (reset) {
                self.register_a = 0;
                self.register_x = 0;
                self.register_y = 0;
                self.status = 0x00 as u8;
            }

            self.run();
        }

        fn fetch(&mut self) -> u8 {
            let data = self.memory[self.program_counter as usize];
            self.program_counter += 1;
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
                AddressingMode::Immediate => self.program_counter,
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
            self.set_flag(Flag::C, old & 0b1000_0000 == 1);
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
            if (!condition) { return; }
            self.program_counter -= 2;
            if rel & 0b1000_0000 == 0 {
                self.program_counter += (rel & 0b0111_1111) as u16;
            } else {
                self.program_counter += (rel as u16 | 0b1111_1111_0000_0000);
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
            todo!();
        }

        fn inc(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            let val: u8 = self.mem_read(addr) + 0b0000_0001;
            self.mem_write(addr, val);

            self.set_zero(val);
            self.set_negative(val);
        }

        ld![lda, register_a, ldx, register_x, ldy, register_y];

        pub fn run(&mut self) {
            // Upon receiving a code, we want to find out the operation and the addressing mode.

            loop {
                let opcode = self.fetch();
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
                    0x0a => todo!(),
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
                    // 4c Absolute
                    // 6c indirect
                    // jsr - jump to subroutine

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

                    // nop - no operation
                    0xea => todo!(),
                    

                    // TAX
                    0xaa => {
                        self.register_x = self.register_a;
                        self.set_flag(Flag::Z, self.register_x == 0);
                        self.set_flag(Flag::N, self.register_x & 0b1000_0000 != 0);
                    },
                    // INX
                    0xe8 => {
                        self.register_x += 1;
                        self.set_flag(Flag::Z, self.register_x == 0);
                        self.set_flag(Flag::N, self.register_x & 0b1000_0000 != 0);
                    },
                    0x00 => return,
                    _ => todo!(""),
                }
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use rand::prelude::*;

        macro_rules! run_test {
            ($instr: ident, $($mode: ident),+) => {
                mod $instr {
                    use super::*;
                    use rand::prelude::*;

                    $(#[test]
                    fn $mode() {
                        let mut cpu = CPU::new();
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

        fn adc(cpu: &mut CPU, mode: AddressingMode, rng: &mut ThreadRng) {
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

        fn and(cpu: &mut CPU, mode: AddressingMode, rng: &mut ThreadRng) {
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

        fn asl(cpu: &mut CPU, mode: AddressingMode, rng: &mut ThreadRng) {
            let mem_value: u8 = next_u8(rng);
            addressing_mode_tester(cpu, mem_value, &mode);

            cpu.asl(mode);

            //assert_eq!(cpu.register_a, );
            assert_eq!(cpu.get_flag(Flag::Z), mem_value << 1 == 0);
            assert_eq!(cpu.get_flag(Flag::N), (mem_value << 1)  & 0b1000_0000 != 0);
        }

        run_test![
            asl,
            ZeroPage,
            ZeroPageX,
            Absolute,
            AbsoluteX
        ];

        fn bit(cpu: &mut CPU, mode: AddressingMode, rng: &mut ThreadRng) {
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
            let mut cpu = CPU::new();
            
            cpu.program_counter = 0x8001;
            cpu.mem_write(0x8001, 0b1001_0101);
            cpu.jump_rel(true);
            assert_eq!(cpu.program_counter, 0x7f95);

            cpu.program_counter = 0x8001;
            cpu.mem_write(0x8001, 0b0110_0101);
            cpu.jump_rel(true);
            assert_eq!(cpu.program_counter, 0x8065); 
        }

        /*  ** Checking jump instructions **
            In the different test methods we set up the cpu flags according to the instruction tested, and call the jump_check 
            method. This method loads the cpu memory with the instruction tested, and a jump pattern, which allows to test
            if the cpu branched or not. We compare this with the expected behavior.
        */

        fn jump_check(instruction: u8, cpu: &mut CPU) -> bool {
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![instruction, 0x03, 0x00], false);
            if (cpu.program_counter == 0x8003) { false }
            else { true }
        }

        #[test]
        fn test_bcc_0x90() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::C, false);
            assert_eq!(jump_check(0x90, &mut cpu), true);

            cpu.set_flag(Flag::C, true);
            assert_eq!(jump_check(0x90, &mut cpu), false);
        }

        #[test]
        fn test_bcs_0xb0() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::C, false);
            assert_eq!(jump_check(0xb0, &mut cpu), false);

            cpu.set_flag(Flag::C, true);
            assert_eq!(jump_check(0xb0, &mut cpu), true);
        }

        #[test]
        fn test_beq_0xf0() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::Z, false);
            assert_eq!(jump_check(0xf0, &mut cpu), false);

            cpu.set_flag(Flag::Z, true);
            assert_eq!(jump_check(0xf0, &mut cpu), true);
        }
        
        #[test]
        fn test_bne_0xd0() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::Z, false);
            assert_eq!(jump_check(0xd0, &mut cpu), true);

            cpu.set_flag(Flag::Z, true);
            assert_eq!(jump_check(0xd0, &mut cpu), false);
        }

        #[test]
        fn test_bmi_0x30() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::N, false);
            assert_eq!(jump_check(0x30, &mut cpu), false);

            cpu.set_flag(Flag::N, true);
            assert_eq!(jump_check(0x30, &mut cpu), true);
        }

        #[test]
        fn test_bpl_0x10() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::N, false);
            assert_eq!(jump_check(0x10, &mut cpu), true);

            cpu.set_flag(Flag::N, true);
            assert_eq!(jump_check(0x10, &mut cpu), false);
        }

        #[test]
        fn test_bvc_0x50() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::V, false);
            assert_eq!(jump_check(0x50, &mut cpu), true);

            cpu.set_flag(Flag::V, true);
            assert_eq!(jump_check(0x50, &mut cpu), false);
        }

        #[test]
        fn test_bvc_0x70() {
            let mut cpu = CPU::new();

            cpu.set_flag(Flag::V, false);
            assert_eq!(jump_check(0x70, &mut cpu), false);

            cpu.set_flag(Flag::V, true);
            assert_eq!(jump_check(0x70, &mut cpu), true);
        }

        fn dec(cpu: &mut CPU, mode: AddressingMode, rng: &mut ThreadRng) {
            let mem_value: u8 = next_u8(rng);

            let addr = addressing_mode_tester(cpu, mem_value, &mode);
            cpu.dec(mode);

            let new_value = mem_value - 1;

            assert_eq!(cpu.mem_read(addr), new_value);
            assert_eq!(cpu.get_flag(Flag::Z), new_value == 0);
            assert_eq!(cpu.get_flag(Flag::N), (mem_value) & 0b1000_0000 != 0);
        }

        run_test![
            dec,
            ZeroPage,
            ZeroPageX,
            Absolute,
            AbsoluteX
        ];

        // what does inc do? well, it increments a memory address...
        fn inc(cpu: &mut CPU, mode: AddressingMode, rng: &mut ThreadRng) {
            let val = next_u8(rng);
            let addr = addressing_mode_tester(cpu, val, &mode);

            cpu.inc(mode);

            let new_val = cpu.mem_read(addr);
            assert_eq!(new_val, val+1);
            assert_eq!(cpu.get_flag(Flag::Z), new_val == 0);
            // check flags
        }

        run_test![inc, ZeroPage, ZeroPageX, Absolute, AbsoluteX];

        //test_inc![ZeroPage, ZeroPageX, Absolute, AbsoluteX];
        

        // Given a cpu and an addressing mode, this method plants a random number in a pre-defined location according to the indexing procedure, and generates code to to access the hidden information.
        fn addressing_mode_tester(cpu: &mut CPU, secret_value: u8, mode: &AddressingMode) -> u16 {
            let lsb: u8 = 10;
            let msb: u8 = 13;
            let addr: u16 = (msb as u16) << 8 + (lsb as u16);
            let reg: u8 = 53;
            let indirect: u16 = 745;

            cpu.program_counter = 0;

            match mode {
                AddressingMode::Immediate => { // this needs to be corrected...
                    cpu.mem_write(cpu.program_counter, secret_value);
                    0 as u16 
                }
                AddressingMode::ZeroPage => {
                    cpu.mem_write(lsb as u16, secret_value);
                    cpu.mem_write(cpu.program_counter, lsb);
                    lsb as u16
                }
                AddressingMode::ZeroPageX => {
                    cpu.register_x = reg;
                    cpu.mem_write(lsb as u16 + reg as u16, secret_value);
                    cpu.mem_write(cpu.program_counter, lsb);
                    lsb as u16 + reg as u16
                }
                AddressingMode::ZeroPageY => {
                    cpu.register_y = reg;
                    cpu.mem_write(lsb as u16 + reg as u16, secret_value);
                    cpu.mem_write(cpu.program_counter, lsb);
                    lsb as u16
                }
                AddressingMode::Absolute => {
                    cpu.mem_write(addr, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    addr
                }
                AddressingMode::AbsoluteX => {
                    cpu.register_x = reg;
                    cpu.mem_write(addr + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::AbsoluteY => {
                    cpu.register_y = reg;
                    cpu.mem_write(addr + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::Indirect => {
                    cpu.mem_write_u16(addr, indirect);
                    cpu.mem_write(indirect, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    indirect
                }
                AddressingMode::IndexedIndirectX => {
                    cpu.register_x = reg;
                    cpu.mem_write_u16(addr + reg as u16, indirect);
                    cpu.mem_write(indirect, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::IndexedIndirectY => {
                    cpu.register_y = reg;
                    cpu.mem_write_u16(addr + reg as u16, indirect);
                    cpu.mem_write(indirect, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    addr + (reg as u16)
                }
                AddressingMode::IndirectIndexedX => {
                    cpu.register_x = reg;
                    cpu.mem_write_u16(addr, indirect);
                    cpu.mem_write(indirect + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    indirect + (reg as u16)
                }
                AddressingMode::IndirectIndexedY => {
                    cpu.register_y = reg;
                    cpu.mem_write_u16(addr, indirect);
                    cpu.mem_write(indirect + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                    indirect + (reg as u16)
                }
            }
        }
    }
}
