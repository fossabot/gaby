use crate::cpu::{
    operands::{Indirect, Source, Target, WordRegister},
    CPUMode, Flags, ReadImmediate, CPU,
};
use std::fmt::{Display, Formatter};

pub enum Condition {
    Unconditional,
    Zero(bool),
    Carry(bool),
}

impl Condition {
    fn is_satisfied(&self, cpu: &CPU) -> bool {
        match self {
            Condition::Unconditional => true,
            Condition::Zero(flag) => cpu.reg.z_flag() == *flag,
            Condition::Carry(flag) => cpu.reg.c_flag() == *flag,
        }
    }
}

impl Display for Condition {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let string = match self {
            Condition::Unconditional => "",
            Condition::Zero(flag) => {
                if *flag {
                    "Z"
                } else {
                    "NZ"
                }
            }
            Condition::Carry(flag) => {
                if *flag {
                    "C"
                } else {
                    "NC"
                }
            }
        };
        write!(f, "{}", string)
    }
}

impl CPU {
    /// ADC
    pub fn add_with_carry(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "ADC ".to_string() + &byte.to_string();

        let (mut sum, mut overflow) = self.reg.a.overflowing_add(byte.read(self));
        if self.reg.c_flag() {
            let (sum_2, overflow_2) = sum.overflowing_add(1);
            sum = sum_2;
            overflow |= overflow_2;
        }
        self.reg.a = sum;

        let mut flags = if self.reg.a == 0 {
            Flags::Z
        } else {
            Flags::empty()
        };
        // FIXME: H is wrong.
        if overflow {
            flags.insert(Flags::C);
        }

        self.reg.set_flags(flags);
    }

    /// ADD
    pub fn add_byte(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "ADD ".to_string() + &byte.to_string();

        let (sum, overflow) = self.reg.a.overflowing_add(byte.read(self));
        self.reg.a = sum;

        let mut flags = if self.reg.a == 0 {
            Flags::Z
        } else {
            Flags::empty()
        };
        // FIXME: H is wrong.
        if overflow {
            flags.insert(Flags::C);
        }

        self.reg.set_flags(flags);
    }

    /// ADD
    pub fn add_word(&mut self, target: impl Source<u16> + Target<u16>, source: impl Source<u16>) {
        self.curr_instr = "ADD ".to_string() + &target.to_string() + ", " + &source.to_string();

        let (sum, overflow) = source.read(self).overflowing_add(target.read(self));
        target.write(self, sum);

        // FIXME: Flags are wrong.
        let mut flags = self.reg.flags();
        flags.remove(Flags::N);
        flags.set(Flags::H, false); // FIXME: Wrong.
        flags.set(Flags::C, overflow);
        self.reg.set_flags(flags);
    }

    /// AND
    pub fn and(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "AND ".to_string() + &byte.to_string();

        self.reg.a &= byte.read(self);

        let flags = if self.reg.a == 0 {
            Flags::Z | Flags::N
        } else {
            Flags::N
        };
        self.reg.set_flags(flags);
    }

    /// BIT
    pub fn test_bit(&mut self, target_bit: u8, data: impl Source<u8>) {
        self.curr_instr = "BIT ".to_string() + &target_bit.to_string() + ", " + &data.to_string();

        let byte = data.read(self);

        let mask = 1 << target_bit;

        let mut flags = self.reg.flags();
        flags.set(Flags::Z, (byte & mask) == 0);
        flags.remove(Flags::N);
        flags.insert(Flags::H);
        self.reg.set_flags(flags);
    }

    /// CALL
    pub fn call(&mut self, word: impl Source<u16>, cond: Condition) {
        let instr = "CALL".to_string() + &cond.to_string() + " " + &word.to_string();

        let address = word.read(self);

        if cond.is_satisfied(self) {
            self.push(WordRegister::PC);

            self.cycles_until_done += 1;
            self.reg.pc = address;
        }

        self.curr_instr = instr;
    }

    /// CP
    pub fn compare(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "CP ".to_string() + &byte.to_string();

        let data = byte.read(self);

        let mut flags = self.reg.flags();
        flags.set(Flags::Z, self.reg.a == data);
        flags.insert(Flags::N);
        flags.set(Flags::H, false); // FIXME: Wrong.
        flags.set(Flags::C, self.reg.a < data);
        self.reg.set_flags(flags);
    }

    /// CPLA
    pub fn complement_a(&mut self) {
        self.curr_instr = "CPLA".to_string();

        self.reg.a = !self.reg.a;

        let mut flags = self.reg.flags();
        flags.insert(Flags::N);
        flags.insert(Flags::H);
        self.reg.set_flags(flags);
    }

    /// DEC
    pub fn decrement_byte(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "DEC ".to_string() + &data.to_string();

        let result = data.read(self).wrapping_sub(1);
        data.write(self, result);

        let mut flags = self.reg.flags();
        flags.set(Flags::Z, result == 0);
        flags.insert(Flags::N);
        flags.set(Flags::H, (result & 0x0F) == 0x0F);
        self.reg.set_flags(flags);
    }

    /// DEC
    pub fn decrement_word(&mut self, data: impl Source<u16> + Target<u16>) {
        self.curr_instr = "DEC ".to_string() + &data.to_string();

        let result = data.read(self).wrapping_sub(1);
        data.write(self, result);

        self.cycles_until_done += 1;
    }

    /// DI
    pub fn disable_interrupts(&mut self) {
        self.curr_instr = "DI".to_string();
        self.ime = false;
    }

    /// EI
    pub fn enable_interrupts(&mut self) {
        self.curr_instr = "EI".to_string();
        self.ime = true;
    }

    /// HALT
    pub fn halt(&mut self) {
        self.mode = CPUMode::Halt;
    }

    /// INC
    pub fn increment_byte(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "INC ".to_string() + &data.to_string();

        let result = data.read(self).wrapping_add(1);
        data.write(self, result);

        let mut flags = self.reg.flags();
        flags.set(Flags::Z, result == 0);
        flags.remove(Flags::N);
        flags.set(Flags::H, result.trailing_zeros() >= 4);
        self.reg.set_flags(flags);
    }

    /// INC
    pub fn increment_word(&mut self, data: impl Source<u16> + Target<u16>) {
        self.curr_instr = "INC ".to_string() + &data.to_string();

        let result = data.read(self).wrapping_add(1);
        data.write(self, result);

        self.cycles_until_done += 1;
    }

    /// JP
    pub fn jump(&mut self, word: impl Source<u16>, cond: Condition) {
        self.curr_instr = "JP".to_string() + &cond.to_string() + " " + &word.to_string();

        let address = word.read(self);

        if cond.is_satisfied(self) {
            self.cycles_until_done += 1;
            self.reg.pc = address;
        }
    }

    /// JR
    pub fn jump_relative(&mut self, cond: Condition) {
        self.curr_instr = "JR".to_string() + &cond.to_string() + " ";

        let immediate: u8 = self.immediate().0;
        let offset = immediate as i8;
        self.curr_instr += &format!("{}", offset);

        if cond.is_satisfied(self) {
            self.cycles_until_done += 1;
            self.reg.pc = (i32::from(self.reg.pc) + i32::from(offset)) as u16;
        }
    }

    /// LD
    pub fn load<T, U: Target<T>, V: Source<T>>(&mut self, target: U, source: V) {
        self.curr_instr = "LD ".to_string() + &target.to_string() + ", " + &source.to_string();

        let data = source.read(self);
        target.write(self, data);
    }

    /// LDD
    pub fn load_and_decrement_hl<T>(&mut self, target: impl Target<T>, source: impl Source<T>) {
        let instr = "LDD ".to_string() + &target.to_string() + ", " + &source.to_string();

        self.load(target, source);
        self.decrement_word(WordRegister::HL);

        self.cycles_until_done -= 1;
        self.curr_instr = instr;
    }

    /// LDI
    pub fn load_and_increment_hl<T>(&mut self, target: impl Target<T>, source: impl Source<T>) {
        let instr = "LDI ".to_string() + &target.to_string() + ", " + &source.to_string();

        self.load(target, source);
        self.increment_word(WordRegister::HL);

        self.cycles_until_done -= 1;
        self.curr_instr = instr;
    }

    /// NOP
    pub fn no_operation(&mut self) {
        self.curr_instr = "NOP".into();
    }

    /// OR
    pub fn or(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "OR ".to_string() + &byte.to_string();

        self.reg.a |= byte.read(self);

        let flags = if self.reg.a == 0 {
            Flags::Z
        } else {
            Flags::empty()
        };
        self.reg.set_flags(flags);
    }

    /// POP
    pub fn pop(&mut self, target: impl Target<u16>) {
        let instr = "POP ".to_string() + &target.to_string();

        self.load(target, Indirect::SP);
        self.reg.sp = self.reg.sp.wrapping_add(2);

        self.curr_instr = instr;
    }

    /// PUSH
    pub fn push(&mut self, source: impl Source<u16>) {
        let instr = "PUSH ".to_string() + &source.to_string();

        self.reg.sp = self.reg.sp.wrapping_sub(2);
        self.load(Indirect::SP, source);

        self.curr_instr = instr;
    }

    /// RES
    pub fn reset_bit(&mut self, target_bit: u8, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "RES ".to_string() + &target_bit.to_string() + ", " + &data.to_string();

        let byte = data.read(self);
        let mask = !(1 << target_bit);
        data.write(self, byte & mask);
    }

    /// RET
    pub fn r#return(&mut self, cond: Condition) {
        let instr = "RET".to_string() + &cond.to_string();

        if cond.is_satisfied(self) {
            self.pop(WordRegister::PC);
            self.cycles_until_done += 1;
        }

        self.curr_instr = instr;
    }

    /// RETI
    pub fn return_and_enable_interrupts(&mut self) {
        let instr = "RETI".to_string();

        self.r#return(Condition::Unconditional);
        self.enable_interrupts();

        self.curr_instr = instr;
    }

    /// RL
    pub fn rotate_left_through_carry(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "RL ".to_string() + &data.to_string();

        let (mut byte, overflow) = data.read(self).overflowing_shl(1);
        if self.reg.c_flag() {
            byte |= 0b0000_0001;
        }
        data.write(self, byte);

        let mut flags = if byte == 0 { Flags::Z } else { Flags::empty() };
        flags.set(Flags::C, overflow);
        self.reg.set_flags(flags);
    }

    /// RR
    pub fn rotate_right_through_carry(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "RR ".to_string() + &data.to_string();

        let (mut byte, overflow) = data.read(self).overflowing_shr(1);
        if self.reg.c_flag() {
            byte |= 0b1000_0000;
        }
        data.write(self, byte);

        let mut flags = if byte == 0 { Flags::Z } else { Flags::empty() };
        flags.set(Flags::C, overflow);
        self.reg.set_flags(flags);
    }

    /// RLC
    pub fn rotate_left(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "RLC ".to_string() + &data.to_string();

        let byte = data.read(self).rotate_left(1);
        data.write(self, byte);

        let mut flags = if byte == 0 { Flags::Z } else { Flags::empty() };
        flags.set(Flags::C, (byte & 0b0000_0001) != 0);
        self.reg.set_flags(flags);
    }

    /// RRC
    pub fn rotate_right(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "RRC ".to_string() + &data.to_string();

        let byte = data.read(self).rotate_right(1);
        data.write(self, byte);

        let mut flags = if byte == 0 { Flags::Z } else { Flags::empty() };
        flags.set(Flags::C, (byte & 0b1000_0000) != 0);
        self.reg.set_flags(flags);
    }

    /// RST
    pub fn restart(&mut self, address: u8) {
        let instr = format!("RST {:#04X}", address);

        self.push(WordRegister::PC);
        self.cycles_until_done += 1;
        self.reg.pc = u16::from(address);

        self.curr_instr = instr;
    }

    /// SBC
    pub fn subtract_with_carry(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "SBC ".to_string() + &byte.to_string();

        let (mut difference, mut overflow) = self.reg.a.overflowing_sub(byte.read(self));
        if self.reg.c_flag() {
            let (difference_2, overflow_2) = difference.overflowing_sub(1);
            difference = difference_2;
            overflow |= overflow_2;
        }
        self.reg.a = difference;

        let mut flags = if self.reg.a == 0 {
            Flags::Z | Flags::N
        } else {
            Flags::N
        };
        // FIXME: H is wrong.
        if !overflow {
            flags.insert(Flags::C);
        }

        self.reg.set_flags(flags);
    }

    /// SCF
    pub fn set_carry_flag(&mut self) {
        self.curr_instr = "SCF".to_string();

        let mut flags = self.reg.flags();
        flags.insert(Flags::C);
        flags.remove(Flags::H);
        flags.remove(Flags::N);
        self.reg.set_flags(flags);
    }

    /// SET
    pub fn set_bit(&mut self, target_bit: u8, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "SET ".to_string() + &target_bit.to_string() + ", " + &data.to_string();

        let byte = data.read(self);
        let mask = 1 << target_bit;
        data.write(self, byte | mask);
    }

    /// SLA
    pub fn shift_left(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "SLA ".to_string() + &data.to_string();

        let (byte, overflow) = data.read(self).overflowing_shl(1);
        data.write(self, byte);

        let mut flags = if byte == 0 { Flags::Z } else { Flags::empty() };
        flags.set(Flags::C, overflow);
        self.reg.set_flags(flags);
    }

    /// SRA
    pub fn shift_right_keep_msb(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "SRA ".to_string() + &data.to_string();

        let (mut byte, overflow) = data.read(self).overflowing_shr(1);
        byte |= (byte & 0b0100_0000) << 1;
        data.write(self, byte);

        let mut flags = if byte == 0 { Flags::Z } else { Flags::empty() };
        flags.set(Flags::C, overflow);
        self.reg.set_flags(flags);
    }

    /// SRL
    pub fn shift_right(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "SRL ".to_string() + &data.to_string();

        let (byte, overflow) = data.read(self).overflowing_shr(1);
        data.write(self, byte);

        let mut flags = if byte == 0 { Flags::Z } else { Flags::empty() };
        flags.set(Flags::C, overflow);
        self.reg.set_flags(flags);
    }

    /// SUB
    pub fn subtract(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "SUB ".to_string() + &byte.to_string();

        let (difference, overflow) = self.reg.a.overflowing_sub(byte.read(self));
        self.reg.a = difference;

        let mut flags = if self.reg.a == 0 {
            Flags::Z | Flags::N
        } else {
            Flags::N
        };
        // FIXME: H is wrong.
        if !overflow {
            flags.insert(Flags::C);
        }

        self.reg.set_flags(flags);
    }

    /// SWAP
    pub fn swap(&mut self, data: impl Source<u8> + Target<u8>) {
        self.curr_instr = "SWAP ".to_string() + &data.to_string();

        let byte = data.read(self);
        let low_nibble = byte & 0b0000_1111;
        let high_nibble = byte & 0b1111_0000;

        let swapped = (low_nibble << 4) & (high_nibble >> 4);
        data.write(self, swapped);

        let flags = if swapped == 0 {
            Flags::Z
        } else {
            Flags::empty()
        };
        self.reg.set_flags(flags);
    }

    /// XOR
    pub fn xor(&mut self, byte: impl Source<u8>) {
        self.curr_instr = "XOR ".to_string() + &byte.to_string();

        self.reg.a ^= byte.read(self);

        let flags = if self.reg.a == 0 {
            Flags::Z
        } else {
            Flags::empty()
        };
        self.reg.set_flags(flags);
    }
}
