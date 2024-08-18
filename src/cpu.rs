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

        // Loads a byte of memory into the accumulator setting the zero and negative flags as appropriate.
        // sets: Zero, Negative
        fn lda(&mut self, mode: AddressingMode) {
            let addr: u16 = self.get_target_address(mode);
            self.register_a = self.mem_read(addr);
            self.set_zero(self.register_a);
            self.set_negative(self.register_a)
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

                    // lda
                    0xa9 => self.lda(AddressingMode::Immediate),
                    0xa5 => self.lda(AddressingMode::ZeroPage),
                    0xb5 => self.lda(AddressingMode::ZeroPageX),
                    0xad => self.lda(AddressingMode::Absolute),
                    0xbd => self.lda(AddressingMode::AbsoluteX),
                    0xb9 => self.lda(AddressingMode::AbsoluteY),
                    0xa1 => self.lda(AddressingMode::IndexedIndirectY),
                    0xb1 => self.lda(AddressingMode::IndirectIndexedY),

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
                    // cmp
                    0xc9 => todo!(), // Immediate
                    0xc5 => todo!(), // Zeropage
                    0xd5 => todo!(), // Zeropage,X
                    0xcd => todo!(), // Absolute

                    // cpx
                    // cpy


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

        ///This test loads a value into A, and checks if the value and the flags are correctly set.

        #[ignore]
        #[test]
        fn test_0xa9_lda_immediate_load_data() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0x05, 0x00], true);
            assert_eq!(cpu.register_a, 0x05);
            assert_eq!(cpu.status & Flag::Z as u8, 0);
            assert_eq!(cpu.status & Flag::N as u8, 0);
        }

        #[ignore]
        #[test]
        fn test_0xa9_lda_zero_flag() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0x00, 0x00], true);
            assert_eq!(cpu.status & Flag::Z as u8, Flag::Z as u8);
            assert_eq!(cpu.status & Flag::N as u8, 0);
        }

        #[ignore]
        #[test]
        fn test_0xa9_lda_negative_flag() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0b1000_0000, 0x00], true);
            assert_eq!(cpu.status & Flag::Z as u8, 0);
            assert_eq!(cpu.status & Flag::N as u8, Flag::N as u8);
        }

        #[ignore]
        #[test]
        fn test_0xaa_tax_load_data() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0x05, 0xaa, 0x00], true);
            assert_eq!(cpu.register_x, 0x05);
            assert_eq!(cpu.status & Flag::Z as u8, 0);
            assert_eq!(cpu.status & Flag::N as u8, 0);
        }

        #[ignore]
        #[test]
        fn test_0xaa_tax_zero_flag() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0x00, 0xaa, 0x00], true);
            assert_eq!(cpu.status & Flag::Z as u8, Flag::Z as u8);
            assert_eq!(cpu.status & Flag::N as u8, 0);
        }

        #[ignore]
        #[test]
        fn test_0xaa_tax_negative_flag() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0b1000_0000, 0xaa, 0x00], true);
            assert_eq!(cpu.status & Flag::Z as u8, 0);
            assert_eq!(cpu.status & Flag::N as u8, Flag::N as u8);
        }

        #[ignore]
        #[test]
        fn test_0xe8_inx_increment() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0x05, 0xaa, 0xe8, 0x00], true);
            assert_eq!(cpu.status & Flag::Z as u8, 0);
            assert_eq!(cpu.status & Flag::N as u8, 0);
        }

        #[ignore]
        #[test]
        fn test_5_ops_working_together() {
            let mut cpu = CPU::new();
            cpu.mem_write_u16(0xfffc, 0x8000);
            cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00], true);
            assert_eq!(cpu.register_x, 0xc1)
        }

        macro_rules! test_adc {
            ($( $x:ident ),*) => {
                mod adc {
                    use super::*;
                    use rand::prelude::*;
                    $(
                        #[test]
                        fn $x() {
                            let mut cpu = CPU::new();
                            let mut rng = rand::thread_rng();
                            let mode = AddressingMode::$x;

                            let rand: u32 = rng.next_u32();

                            let a: u8 = (rand & 0xff) as u8;
                            let c: u8 = ((rand >> 8) & 0xff) as u8 % 2;
                            let secret_value: u8 = ((rand >> 16) & 0xff) as u8;

                            cpu.register_a = a;
                            cpu.set_flag(Flag::C, c != 0);
                            addressing_mode_tester(&mut cpu, secret_value, &mode);

                            cpu.adc(mode);
                            assert_eq!(cpu.register_a, a + c + secret_value);
                            assert_eq!(cpu.get_flag(Flag::Z), cpu.register_a == 0);
                            assert_eq!(cpu.get_flag(Flag::N), cpu.register_a  & 0b1000_0000 != 0);
                        }
                    )*
                }
            }
        }

        test_adc![
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

        macro_rules! test_and {
            ($( $x:ident ),*) => {
                mod and {
                    use super::*;
                    use rand::prelude::*;
                    $(
                        #[test]
                        fn $x() {
                            let mut cpu = CPU::new();
                            let mut rng = rand::thread_rng();
                            let mode = AddressingMode::$x;

                            let rand: u32 = rng.next_u32();

                            let a: u8 = (rand & 0xff) as u8;
                            let secret_value: u8 = ((rand >> 16) & 0xff) as u8;

                            cpu.register_a = a;
                            addressing_mode_tester(&mut cpu, secret_value, &mode);

                            cpu.and(mode);
                            assert_eq!(cpu.register_a, a & secret_value);
                            assert_eq!(cpu.get_flag(Flag::Z), cpu.register_a == 0);
                            assert_eq!(cpu.get_flag(Flag::N), cpu.register_a  & 0b1000_0000 != 0);
                        }
                    )*
                }
            }
        }

        test_and![
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

        macro_rules! test_asl {
            ($( $x:ident ),*) => {
                mod asl {
                    use super::*;
                    use rand::prelude::*;
                    $(
                        #[test]
                        fn $x() {
                            let mut cpu = CPU::new();
                            let mut rng = rand::thread_rng();
                            let mode = AddressingMode::$x;

                            let rand: u32 = rng.next_u32();

                            let secret_value: u8 = ((rand >> 16) & 0xff) as u8;
                            addressing_mode_tester(&mut cpu, secret_value, &mode);

                            cpu.asl(mode);
                            //assert_eq!(cpu.register_a, );
                            assert_eq!(cpu.get_flag(Flag::Z), secret_value << 1 == 0);
                            assert_eq!(cpu.get_flag(Flag::N), (secret_value << 1)  & 0b1000_0000 != 0);
                        }
                    )*
                }
            }
        }

        test_asl![
            ZeroPage,
            ZeroPageX,
            Absolute,
            AbsoluteX
        ];

        macro_rules! test_bit {
            ($($x: ident), * ) => {
                mod bit {
                    use super::*;
                    use rand::prelude::*;
                    $(
                        #[test]
                        fn $x() {
                            let mut cpu = CPU::new();
                            let mut rng = rand::thread_rng();
                            let mode = AddressingMode::$x;

                            let rand: u32 = rng.next_u32();

                            let reg: u8 = (rand & 0xff) as u8;
                            let secret_value: u8 = ((rand >> 16) & 0xff) as u8;

                            cpu.register_a = reg;
                            addressing_mode_tester(&mut cpu, secret_value, &mode);

                            cpu.bit(mode);

                            // check that the Zero flag contains the result.
                            
                            assert_eq!(cpu.get_flag(Flag::Z), reg & secret_value == 0);
                            // Check that the N and V flags are correctly set.
                            assert_eq!(cpu.get_flag(Flag::N), secret_value & 0b1000_0000 != 0);
                            assert_eq!(cpu.get_flag(Flag::V), secret_value & 0b0100_0000 != 0);
                        } 
                    )*
                }
            }
        }

        test_bit![ZeroPage, Absolute];


        /*macro_rules! test_rel_jumps {
            ($($x: ident), * ) => {

            }
        }*/

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
        

        // Given a cpu and an addressing mode, this method plants a random number in a pre-defined location according to the indexing procedure, and generates code to to access the hidden information.
        fn addressing_mode_tester(cpu: &mut CPU, secret_value: u8, mode: &AddressingMode) {
            let lsb: u8 = 10;
            let msb: u8 = 13;
            let addr: u16 = (msb as u16) << 8 + (lsb as u16);
            let reg: u8 = 53;
            let indirect: u16 = 745;

            cpu.program_counter = 0;

            match mode {
                AddressingMode::Immediate => {
                    cpu.mem_write(cpu.program_counter, secret_value);
                }
                AddressingMode::ZeroPage => {
                    cpu.mem_write(lsb as u16, secret_value);
                    cpu.mem_write(cpu.program_counter, lsb);
                }
                AddressingMode::ZeroPageX => {
                    cpu.register_x = reg;
                    cpu.mem_write(lsb as u16 + reg as u16, secret_value);
                    cpu.mem_write(cpu.program_counter, lsb);
                }
                AddressingMode::ZeroPageY => {
                    cpu.register_y = reg;
                    cpu.mem_write(lsb as u16 + reg as u16, secret_value);
                    cpu.mem_write(cpu.program_counter, lsb);
                }
                AddressingMode::Absolute => {
                    cpu.mem_write(addr, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
                AddressingMode::AbsoluteX => {
                    cpu.register_x = reg;
                    cpu.mem_write(addr + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
                AddressingMode::AbsoluteY => {
                    cpu.register_y = reg;
                    cpu.mem_write(addr + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
                AddressingMode::Indirect => {
                    cpu.mem_write_u16(addr, indirect);
                    cpu.mem_write(indirect, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
                AddressingMode::IndexedIndirectX => {
                    cpu.register_x = reg;
                    cpu.mem_write_u16(addr + reg as u16, indirect);
                    cpu.mem_write(indirect, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
                AddressingMode::IndexedIndirectY => {
                    cpu.register_y = reg;
                    cpu.mem_write_u16(addr + reg as u16, indirect);
                    cpu.mem_write(indirect, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
                AddressingMode::IndirectIndexedX => {
                    cpu.register_x = reg;
                    cpu.mem_write_u16(addr, indirect);
                    cpu.mem_write(indirect + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
                AddressingMode::IndirectIndexedY => {
                    cpu.register_y = reg;
                    cpu.mem_write_u16(addr, indirect);
                    cpu.mem_write(indirect + reg as u16, secret_value);
                    cpu.mem_write_u16(cpu.program_counter, addr);
                }
            };
        }
    }
}
