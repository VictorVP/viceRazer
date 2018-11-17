use memory::*;
use std::fs::OpenOptions;
use std::fs::File;
use std::io::prelude::*;

const C: u8 = 0x01;
const N: u8 = 0x02;
const P_V: u8 = 0x04;
const BIT_3: u8 = 0x08;
const H: u8 = 0x10;
const BIT_5: u8 = 0x20;
const Z: u8 = 0x40;
const S: u8 = 0x80;

enum OpCodePrefix {
    None,
    DD,
    FD,
    CB,
    FdCb,
    DdCb,
    ED,
}

pub struct Z80 {
    pub halt: bool,
    pc: u16,
    sp: u16,
    ix_h: u8,
    ix_l: u8,
    iy_h: u8,
    iy_l: u8,

    i: u8,
    r: u8,

    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,

    a_alt: u8,
    f_alt: u8,
    b_alt: u8,
    c_alt: u8,
    d_alt: u8,
    e_alt: u8,
    h_alt: u8,
    l_alt: u8,

    iff1: bool,
    iff2: bool,

    halted: bool,
    opcode_prefix: OpCodePrefix,

    log_file: File,
    list_file: File,
}

impl Z80 {
    pub fn new() -> Z80 {
        let out = Z80 {
            halt: false,
            pc: 0,
            sp: 0xFFFF,
            ix_h: 0,
            ix_l: 0,
            iy_h: 0,
            iy_l: 0,
            i: 0,
            r: 0,
            a: 0x00,
            f: 0x00,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            a_alt: 0,
            f_alt: 0,
            b_alt: 0,
            c_alt: 0,
            d_alt: 0,
            e_alt: 0,
            h_alt: 0,
            l_alt: 0,
            iff1: false,
            iff2: false,
            halted: false,
            opcode_prefix: OpCodePrefix::None,
            log_file: OpenOptions::new()
                .append(true)
                .create(true)
                .open("../volcadoEmu/salidaMio.txt")
                .unwrap(),
            list_file: OpenOptions::new()
                .append(true)
                .create(true)
                .open("../volcadoEmu/listado.txt")
                .unwrap(),
        };

        out
    }
    fn get_flag(&self, flag: u8) -> u8 {
        let eval = self.f & flag != 0;
        let result = match eval {
            true => 1,
            false => 0,
        };
        result
    }
    fn set_flag(&mut self, flag: u8) {
        self.f |= flag;
    }
    fn reset_flag(&mut self, flag: u8) {
        self.f &= !flag;
    }
    fn set_reset_flag(&mut self, cond: bool, flag: u8) {
        if cond {
            self.set_flag(flag);
        } else {
            self.reset_flag(flag);
        }
    }
    fn save_state(&mut self) {
        // let mut file = OpenOptions::new()
        //     .append(true)
        //     .create(true)
        //     .open("../volcadoEmu/salidaMio.txt")
        //     .unwrap();

        if let Err(e) = writeln!(
            self.log_file,
            "pc:{} sp:{} ix:{} iy:{} i:{} r:{} af:{} bc:{} de:{} hl:{}",
            format!("{0:01$x}", self.pc, 4),
            format!("{0:01$x}", self.sp, 4),
            format!("{0:01$x}", Z80::get_word(self.ix_h, self.ix_l), 4),
            format!("{0:01$x}", Z80::get_word(self.iy_h, self.iy_l), 4),
            format!("{0:01$x}", self.i, 4),
            format!("{0:01$x}", self.r, 4),
            format!("{0:01$x}", Z80::get_word(self.a, self.f), 4),
            format!("{0:01$x}", Z80::get_word(self.b, self.c), 4),
            format!("{0:01$x}", Z80::get_word(self.d, self.e), 4),
            format!("{0:01$x}", Z80::get_word(self.h, self.l), 4)
        ) {
            eprintln!("No he podido escribir la línea");
        }
    }
    fn save_op(&mut self, msg: &str) {
        // let mut file = OpenOptions::new()
        //     .append(true)
        //     .create(true)
        //     .open("../volcadoEmu/listado.txt")
        //     .unwrap();
        if let Err(e) = writeln!(self.list_file, "{}    {}", format!("{0:01$x}", self.pc, 4), msg) {
            eprintln!("No he podido escribir la línea");
        }

        self.save_state();
    }
    fn get_bytes(val: u16) -> (u8, u8) {
        let hi: u8 = (val >> 8) as u8;
        let lo: u8 = (val & 0xFF) as u8;
        (hi, lo)
    }
    fn get_word(hi: u8, lo: u8) -> u16 {
        ((hi as u16) << 8) + (lo as u16)
    }
    fn check_flag(flag_container: u8, flag: u8) -> bool {
        flag_container & flag != 0
    }
    fn check_byte_parity(byte: u8) -> bool {
        let mut res = true;
        let mut partial = byte;

        while partial > 0 {
            res = match partial & 1 {
                1 => !res,
                0 => res,
                _ => true,
            };
            partial = partial >> 1;
        }

        res
    }
    fn inc_single_register(reg: u8) -> u8 {
        let s = reg as u16;
        ((s + 1) & 0xff) as u8
    }
    fn get_h(&self) -> u8 {
        let res = match self.opcode_prefix {
            OpCodePrefix::DD | OpCodePrefix::DdCb => self.ix_h,
            OpCodePrefix::FD | OpCodePrefix::FdCb => self.iy_h,
            _ => self.h,
        };
        res
    }
    fn get_l(&self) -> u8 {
        let res = match self.opcode_prefix {
            OpCodePrefix::DD | OpCodePrefix::DdCb => self.ix_l,
            OpCodePrefix::FD | OpCodePrefix::FdCb => self.iy_l,
            _ => self.l,
        };
        res
    }
    fn get_hl(&self) -> u16 {
        Z80::get_word(self.get_h(), self.get_l())
    }
    fn set_h(&mut self, val: u8) {
        match self.opcode_prefix {
            OpCodePrefix::DD | OpCodePrefix::DdCb => self.ix_h = val,
            OpCodePrefix::FD | OpCodePrefix::FdCb => self.iy_h = val,
            _ => self.h = val,
        }
    }
    fn set_l(&mut self, val: u8) {
        match self.opcode_prefix {
            OpCodePrefix::DD | OpCodePrefix::DdCb => self.ix_l = val,
            OpCodePrefix::FD | OpCodePrefix::FdCb => self.iy_l = val,
            _ => self.l = val,
        }
    }
    fn get_indirect_hl(&mut self, mem: &Memory) -> u16 {
        let mut addr = Z80::get_word(self.get_h(), self.get_l());
        match self.opcode_prefix {
            OpCodePrefix::DD | OpCodePrefix::FD | OpCodePrefix::FdCb => {
                addr += self.read_bus(mem) as u16
            }
            _ => addr += 0,
        }
        addr
    }
    pub fn exec(&mut self, mem: &mut Memory) {
        let byte = self.read_bus(mem);
        self.r = Z80::inc_single_register(self.r);

        if self.pc == 0x1223 {
            println!("0x1223 El opcode es {:x}", byte);
        }
        if self.pc == 0x1225 {
            println!("0x1225 El opcode es {:x}", byte);
        }
        match self.opcode_prefix {
            OpCodePrefix::None => self.exec_no_prefix(mem, byte),
            OpCodePrefix::DD | OpCodePrefix::FD => self.exec_dd_or_fd_prefix(mem, byte),
            OpCodePrefix::CB => self.exec_cb_prefix(mem, byte),
            OpCodePrefix::FdCb | OpCodePrefix::DdCb => self.exec_fd_cb_prefix(mem, byte),
            OpCodePrefix::ED => self.exec_ed_prefix(mem, byte),
        };
    }
    fn exec_no_prefix(&mut self, mem: &mut Memory, byte: u8) {
        let mut new_prefix = OpCodePrefix::None;
        match byte {
            0x00 => self.nop(),
            0x01 => self.ld_bc(mem),
            0x02 => self.ld_at_bc_a(mem),
            0x03 => self.inc_bc(),
            0x04 => self.inc_b(),
            0x05 => self.dec_b(),
            0x06 => self.ld_b_n(mem),
            0x07 => self.rlca(),
            0x08 => self.ex_af_af_alt(),
            0x09 => self.add_hl_bc(),
            0x0A => self.ld_a_at_bc(mem),
            0x0B => self.dec_bc(),
            0x0C => self.inc_c(),
            0x0D => self.dec_c(),
            0x0E => self.ld_c_n(mem),
            0x0F => self.rrca(),
            0x10 => self.djnz_e(mem),
            0x11 => self.ld_de(mem),
            0x12 => self.ld_at_de_a(mem),
            0x13 => self.inc_de(),
            0x14 => self.inc_d(),
            0x15 => self.dec_d(),
            0x16 => self.ld_d_n(mem),
            0x17 => self.rla(),
            0x18 => self.jr_e(mem),
            0x19 => self.add_hl_de(),
            0x1A => self.ld_a_at_de(mem),
            0x1B => self.dec_de(),
            0x1C => self.inc_e(),
            0x1D => self.dec_e(),
            0x1E => self.ld_e_n(mem),
            0x1F => self.rra(),
            0x20 => self.jr_nz_e(mem),
            0x21 => self.ld_hl(mem),
            0x22 => self.ld_nn_hl(mem),
            0x23 => self.inc_hl(),
            0x24 => self.inc_h(),
            0x25 => self.dec_h(),
            0x26 => self.ld_h_n(mem),
            0x27 => self.daa(),
            0x28 => self.jr_z_e(mem),
            0x29 => self.add_hl_hl(),
            0x2A => self.ld_hl_nn(mem),
            0x2B => self.dec_hl(),
            0x2C => self.inc_l(),
            0x2D => self.dec_l(),
            0x2E => self.ld_l_n(mem),
            0x2F => self.cpl(),
            0x30 => self.jr_nc_e(mem),
            0x31 => self.ld_sp(mem),
            0x32 => self.ld_at_nn_a(mem),
            0x33 => self.inc_sp(),
            0x34 => self.inc_at_hl(mem),
            0x35 => self.dec_at_hl(mem),
            0x36 => self.ld_hl_n(mem),
            0x37 => self.scf(),
            0x38 => self.jr_c_e(mem),
            0x39 => self.add_hl_sp(),
            0x3A => self.ld_a_at_nn(mem),
            0x3B => self.dec_sp(),
            0x3C => self.inc_a(),
            0x3D => self.dec_a(),
            0x3E => self.ld_a_n(mem),
            0x3F => self.ccf(),
            0x40 => self.ld_b_b(),
            0x41 => self.ld_b_c(),
            0x42 => self.ld_b_d(),
            0x43 => self.ld_b_e(),
            0x44 => self.ld_b_h(),
            0x45 => self.ld_b_l(),
            0x46 => self.ld_b_hl(mem),
            0x47 => self.ld_b_a(),
            0x48 => self.ld_c_b(),
            0x49 => self.ld_c_c(),
            0x4A => self.ld_c_d(),
            0x4B => self.ld_c_e(),
            0x4C => self.ld_c_h(),
            0x4D => self.ld_c_l(),
            0x4E => self.ld_c_hl(mem),
            0x4F => self.ld_c_a(),
            0x50 => self.ld_d_b(),
            0x51 => self.ld_d_c(),
            0x52 => self.ld_d_d(),
            0x53 => self.ld_d_e(),
            0x54 => self.ld_d_h(),
            0x55 => self.ld_d_l(),
            0x56 => self.ld_d_hl(mem),
            0x57 => self.ld_d_a(),
            0x58 => self.ld_e_b(),
            0x59 => self.ld_e_c(),
            0x5A => self.ld_e_d(),
            0x5B => self.ld_e_e(),
            0x5C => self.ld_e_h(),
            0x5D => self.ld_e_l(),
            0x5E => self.ld_e_hl(mem),
            0x5F => self.ld_e_a(),
            0x60 => self.ld_h_b(),
            0x61 => self.ld_h_c(),
            0x62 => self.ld_h_d(),
            0x63 => self.ld_h_e(),
            0x64 => self.ld_h_h(),
            0x65 => self.ld_h_l(),
            0x66 => self.ld_h_hl(mem),
            0x67 => self.ld_h_a(),
            0x68 => self.ld_l_b(),
            0x69 => self.ld_l_c(),
            0x6A => self.ld_l_d(),
            0x6B => self.ld_l_e(),
            0x6C => self.ld_l_h(),
            0x6D => self.ld_l_l(),
            0x6E => self.ld_l_hl(mem),
            0x6F => self.ld_l_a(),
            0x70 => self.ld_at_hl_b(mem),
            0x71 => self.ld_at_hl_c(mem),
            0x72 => self.ld_at_hl_d(mem),
            0x73 => self.ld_at_hl_e(mem),
            0x74 => self.ld_at_hl_h(mem),
            0x75 => self.ld_at_hl_l(mem),
            0x76 => self.halt(),
            0x77 => self.ld_at_hl_a(mem),
            0x78 => self.ld_a_b(),
            0x79 => self.ld_a_c(),
            0x7A => self.ld_a_d(),
            0x7B => self.ld_a_e(),
            0x7C => self.ld_a_h(),
            0x7D => self.ld_a_l(),
            0x7E => self.ld_a_at_hl(mem),
            0x7F => self.ld_a_a(),
            0x80 => self.add_a_b(),
            0x81 => self.add_a_c(),
            0x82 => self.add_a_d(),
            0x83 => self.add_a_e(),
            0x84 => self.add_a_h(),
            0x85 => self.add_a_l(),
            0x86 => self.add_a_at_hl(mem),
            0x87 => self.add_a_a(),
            0x88 => self.adc_a_b(),
            0x89 => self.adc_a_c(),
            0x8A => self.adc_a_d(),
            0x8B => self.adc_a_e(),
            0x8C => self.adc_a_h(),
            0x8D => self.adc_a_l(),
            0x8E => self.adc_a_at_hl(mem),
            0x8F => self.adc_a_a(),
            0x90 => self.sub_a_b(),
            0x91 => self.sub_a_c(),
            0x92 => self.sub_a_d(),
            0x93 => self.sub_a_e(),
            0x94 => self.sub_a_h(),
            0x95 => self.sub_a_l(),
            0x96 => self.sub_a_at_hl(mem),
            0x97 => self.sub_a_a(),
            0x98 => self.sbc_a_b(),
            0x99 => self.sbc_a_c(),
            0x9A => self.sbc_a_d(),
            0x9B => self.sbc_a_e(),
            0x9C => self.sbc_a_h(),
            0x9D => self.sbc_a_l(),
            0x9E => self.sbc_a_at_hl(mem),
            0x9F => self.sbc_a_a(),
            0xA0 => self.and_b(),
            0xA1 => self.and_c(),
            0xA2 => self.and_d(),
            0xA3 => self.and_e(),
            0xA4 => self.and_h(),
            0xA5 => self.and_l(),
            0xA6 => self.and_at_hl(mem),
            0xA7 => self.and_a(),
            0xA8 => self.xor_b(),
            0xA9 => self.xor_c(),
            0xAA => self.xor_d(),
            0xAB => self.xor_e(),
            0xAC => self.xor_h(),
            0xAD => self.xor_l(),
            0xAE => self.xor_at_hl(mem),
            0xAF => self.xor_a(),
            0xB0 => self.or_b(),
            0xB1 => self.or_c(),
            0xB2 => self.or_d(),
            0xB3 => self.or_e(),
            0xB4 => self.or_h(),
            0xB5 => self.or_l(),
            0xB6 => self.or_at_hl(mem),
            0xB7 => self.or_a(),
            0xB8 => self.cp_b(),
            0xB9 => self.cp_c(),
            0xBA => self.cp_d(),
            0xBB => self.cp_e(),
            0xBC => self.cp_h(),
            0xBD => self.cp_l(),
            0xBE => self.cp_at_hl(mem),
            0xBF => self.cp_a(),
            0xC0 => self.ret_nz(mem),
            0xC1 => self.pop_bc(mem),
            0xC2 => self.jp_nz(mem),
            0xC3 => self.jp_nn(mem),
            0xC4 => self.call_nz(mem),
            0xC5 => self.push_bc(mem),
            0xC6 => self.add_a_n(mem),
            0xC7 => self.rst_0(mem),
            0xC8 => self.ret_z(mem),
            0xC9 => self.ret(mem),
            0xCA => self.jp_z(mem),
            0xCB => new_prefix = OpCodePrefix::CB,
            0xCC => self.call_z(mem),
            0xCD => self.call_nn(mem),
            0xCE => self.adc_a_n(mem),
            0xCF => self.rst_8(mem),
            0xD0 => self.ret_nc(mem),
            0xD1 => self.pop_de(mem),
            0xD2 => self.jp_nc(mem),
            0xD3 => self.out_n_a(mem),
            0xD4 => self.call_nc(mem),
            0xD5 => self.push_de(mem),
            0xD6 => self.sub_a_n(mem),
            0xD7 => self.rst_10(mem),
            0xD8 => self.ret_c(mem),
            0xD9 => self.exx(),
            0xDA => self.jp_c(mem),
            0xDC => self.call_c(mem),
            0xDD => new_prefix = OpCodePrefix::DD,
            0xDE => self.sbc_a_n(mem),
            0xDF => self.rst_18(mem),
            0xE0 => self.ret_po(mem),
            0xE1 => self.pop_hl(mem),
            0xE2 => self.jp_po(mem),
            0xE3 => self.ex_at_sp_hl(mem),
            0xE4 => self.call_po(mem),
            0xE5 => self.push_hl(mem),
            0xE6 => self.and_n(mem),
            0xE7 => self.rst_20(mem),
            0xE8 => self.ret_pe(mem),
            0xE9 => self.jp_at_hl(),
            0xEA => self.jp_pe(mem),
            0xEB => self.ex_de_hl(),
            0xEC => self.call_pe(mem),
            0xED => new_prefix = OpCodePrefix::ED,
            0xEE => self.xor_n(mem),
            0xEF => self.rst_28(mem),
            0xF0 => self.ret_p(mem),
            0xF1 => self.pop_af(mem),
            0xF2 => self.jp_p(mem),
            0xF3 => self.di(),
            0xF4 => self.call_p(mem),
            0xF5 => self.push_af(mem),
            0xF6 => self.or_n(mem),
            0xF7 => self.rst_30(mem),
            0xF8 => self.ret_m(mem),
            0xF9 => self.ld_sp_hl(),
            0xFA => self.jp_m(mem),
            0xFB => self.ei(),
            0xFC => self.call_m(mem),
            0xFD => new_prefix = OpCodePrefix::FD,
            0xFE => self.cp_n(mem),
            0xFF => self.rst_38(mem),
            _ => {
                println!("El opCode {:x} no está implementado", byte);
                self.halt = true;
            }
        }
        self.opcode_prefix = new_prefix;
    }
    fn exec_dd_or_fd_prefix(&mut self, mem: &mut Memory, byte: u8) {
        let mut new_prefix = OpCodePrefix::None;
        match byte {
            0x09 => self.add_hl_bc(),
            0x19 => self.add_hl_de(),
            0x21 => self.ld_hl(mem),
            0x22 => self.ld_nn_hl(mem),
            0x23 => self.inc_hl(),
            0x24 => self.inc_h(),
            0x25 => self.dec_h(),
            0x26 => self.ld_h_n(mem),
            0x29 => self.add_hl_hl(),
            0x2A => self.ld_hl_nn(mem),
            0x2B => self.dec_hl(),
            0x2C => self.inc_l(),
            0x2D => self.dec_l(),
            0x2E => self.ld_l_n(mem),
            0x34 => self.inc_at_hl(mem),
            0x35 => self.dec_at_hl(mem),
            0x36 => self.ld_hl_n(mem),
            0x39 => self.add_hl_sp(),
            0x44 => self.ld_b_h(),
            0x45 => self.ld_b_l(),
            0x46 => self.ld_b_hl(mem),
            0x4C => self.ld_c_h(),
            0x4D => self.ld_c_l(),
            0x4E => self.ld_c_hl(mem),
            0x54 => self.ld_d_h(),
            0x55 => self.ld_d_l(),
            0x56 => self.ld_d_hl(mem),
            0x5C => self.ld_e_h(),
            0x60 => self.ld_h_b(),
            0x61 => self.ld_h_c(),
            0x62 => self.ld_h_d(),
            0x63 => self.ld_h_e(),
            0x64 => self.ld_h_h(),
            0x65 => self.ld_h_l(),
            0x66 => self.ld_h_hl(mem),
            0x67 => self.ld_h_a(),
            0x68 => self.ld_l_b(),
            0x69 => self.ld_l_c(),
            0x6A => self.ld_l_d(),
            0x6B => self.ld_l_e(),
            0x6C => self.ld_l_h(),
            0x6D => self.ld_l_l(),
            0x6E => self.ld_l_hl(mem),
            0x6F => self.ld_l_a(),
            0x70 => self.ld_at_hl_b(mem),
            0x71 => self.ld_at_hl_c(mem),
            0x72 => self.ld_at_hl_d(mem),
            0x73 => self.ld_at_hl_e(mem),
            0x74 => self.ld_at_hl_h(mem),
            0x75 => self.ld_at_hl_l(mem),
            0x77 => self.ld_at_hl_a(mem),
            0x7C => self.ld_a_h(),
            0x7D => self.ld_a_l(),
            0x7E => self.ld_a_at_hl(mem),
            0x84 => self.add_a_h(),
            0x85 => self.add_a_l(),
            0x86 => self.add_a_at_hl(mem),
            0x8C => self.adc_a_h(),
            0x8D => self.adc_a_l(),
            0x8E => self.adc_a_at_hl(mem),
            0x94 => self.sub_a_h(),
            0x95 => self.sub_a_l(),
            0x96 => self.sub_a_at_hl(mem),
            0x9C => self.sbc_a_h(),
            0x9D => self.sbc_a_l(),
            0x9E => self.sbc_a_at_hl(mem),
            0xA4 => self.and_h(),
            0xA5 => self.and_l(),
            0xA6 => self.and_at_hl(mem),
            0xAC => self.xor_h(),
            0xAD => self.xor_l(),
            0xAE => self.xor_at_hl(mem),
            0xB4 => self.or_h(),
            0xB5 => self.or_l(),
            0xB6 => self.or_at_hl(mem),
            0xBC => self.cp_h(),
            0xBD => self.cp_l(),
            0xBE => self.cp_at_hl(mem),
            0xCB => {
                new_prefix = match self.opcode_prefix {
                    OpCodePrefix::FD => OpCodePrefix::FdCb,
                    _ => OpCodePrefix::DdCb,
                }
            }
            0xDD => new_prefix = OpCodePrefix::DD,
            0xE1 => self.pop_hl(mem),
            0xE3 => self.ex_at_sp_hl(mem),
            0xE5 => self.push_hl(mem),
            0xE9 => self.jp_at_hl(),
            0xED => new_prefix = OpCodePrefix::ED,
            0xFD => new_prefix = OpCodePrefix::FD,
            _ => {
                let prefix = match self.opcode_prefix {
                    OpCodePrefix::DD => "DD",
                    OpCodePrefix::FD => "FD",
                    _ => "otro",
                };
                println!("El opCode {} {:x} no está implementado", prefix, byte);
                self.halt = true;
            }
        }
        self.opcode_prefix = new_prefix;
    }
    fn exec_cb_prefix(&mut self, mem: &mut Memory, byte: u8) {
        match byte {
            0x00 => self.rlc_b(),
            0x01 => self.rlc_c(),
            0x02 => self.rlc_d(),
            0x03 => self.rlc_e(),
            0x04 => self.rlc_h(),
            0x05 => self.rlc_l(),
            0x06 => self.rlc_at_hl(mem),
            0x07 => self.rlc_a(),
            0x08 => self.rrc_b(),
            0x09 => self.rrc_c(),
            0x0A => self.rrc_d(),
            0x0B => self.rrc_e(),
            0x0C => self.rrc_h(),
            0x0D => self.rrc_l(),
            0x0E => self.rrc_at_hl(mem),
            0x0F => self.rrc_a(),
            0x10 => self.rl_b(),
            0x11 => self.rl_c(),
            0x12 => self.rl_d(),
            0x13 => self.rl_e(),
            0x14 => self.rl_h(),
            0x15 => self.rl_l(),
            0x16 => self.rl_at_hl(mem),
            0x17 => self.rl_a(),
            0x18 => self.rr_b(),
            0x19 => self.rr_c(),
            0x1A => self.rr_d(),
            0x1B => self.rr_e(),
            0x1C => self.rr_h(),
            0x1D => self.rr_l(),
            0x1E => self.rr_at_hl(mem),
            0x1F => self.rr_a(),
            0x20 => self.sla_b(),
            0x21 => self.sla_c(),
            0x22 => self.sla_d(),
            0x23 => self.sla_e(),
            0x24 => self.sla_h(),
            0x25 => self.sla_l(),
            0x26 => self.sla_at_hl(mem),
            0x27 => self.sla_a(),
            0x28 => self.sra_b(),
            0x29 => self.sra_c(),
            0x2A => self.sra_d(),
            0x2B => self.sra_e(),
            0x2C => self.sra_h(),
            0x2D => self.sra_l(),
            0x2E => self.sra_at_hl(mem),
            0x2F => self.sra_a(),
            0x30 => self.sll_b(),
            0x31 => self.sll_c(),
            0x32 => self.sll_d(),
            0x33 => self.sll_e(),
            0x34 => self.sll_h(),
            0x35 => self.sll_l(),
            0x36 => self.sll_at_hl(mem),
            0x37 => self.sll_a(),
            0x38 => self.srl_b(),
            0x39 => self.srl_c(),
            0x3A => self.srl_d(),
            0x3B => self.srl_e(),
            0x3C => self.srl_h(),
            0x3D => self.srl_l(),
            0x3E => self.srl_at_hl(mem),
            0x3F => self.srl_a(),
            0x40 => self.bit_0_b(),
            0x41 => self.bit_0_c(),
            0x42 => self.bit_0_d(),
            0x43 => self.bit_0_e(),
            0x44 => self.bit_0_h(),
            0x45 => self.bit_0_l(),
            0x46 => self.bit_0_at_hl(mem),
            0x47 => self.bit_0_a(),
            0x48 => self.bit_1_b(),
            0x49 => self.bit_1_c(),
            0x4A => self.bit_1_d(),
            0x4B => self.bit_1_e(),
            0x4C => self.bit_1_h(),
            0x4D => self.bit_1_l(),
            0x4E => self.bit_1_at_hl(mem),
            0x4F => self.bit_1_a(),
            0x50 => self.bit_2_b(),
            0x51 => self.bit_2_c(),
            0x52 => self.bit_2_d(),
            0x53 => self.bit_2_e(),
            0x54 => self.bit_2_h(),
            0x55 => self.bit_2_l(),
            0x56 => self.bit_2_at_hl(mem),
            0x57 => self.bit_2_a(),
            0x58 => self.bit_3_b(),
            0x59 => self.bit_3_c(),
            0x5A => self.bit_3_d(),
            0x5B => self.bit_3_e(),
            0x5C => self.bit_3_h(),
            0x5D => self.bit_3_l(),
            0x5E => self.bit_3_at_hl(mem),
            0x5F => self.bit_3_a(),
            0x60 => self.bit_4_b(),
            0x61 => self.bit_4_c(),
            0x62 => self.bit_4_d(),
            0x63 => self.bit_4_e(),
            0x64 => self.bit_4_h(),
            0x65 => self.bit_4_l(),
            0x66 => self.bit_4_at_hl(mem),
            0x67 => self.bit_4_a(),
            0x68 => self.bit_5_b(),
            0x69 => self.bit_5_c(),
            0x6A => self.bit_5_d(),
            0x6B => self.bit_5_e(),
            0x6C => self.bit_5_h(),
            0x6D => self.bit_5_l(),
            0x6E => self.bit_5_at_hl(mem),
            0x6F => self.bit_5_a(),
            0x70 => self.bit_6_b(),
            0x71 => self.bit_6_c(),
            0x72 => self.bit_6_d(),
            0x73 => self.bit_6_e(),
            0x74 => self.bit_6_h(),
            0x75 => self.bit_6_l(),
            0x76 => self.bit_6_at_hl(mem),
            0x77 => self.bit_6_a(),
            0x78 => self.bit_7_b(),
            0x79 => self.bit_7_c(),
            0x7A => self.bit_7_d(),
            0x7B => self.bit_7_e(),
            0x7C => self.bit_7_h(),
            0x7D => self.bit_7_l(),
            0x7E => self.bit_7_at_hl(mem),
            0x7F => self.bit_7_a(),
            0x80 => self.res_0_b(),
            0x81 => self.res_0_c(),
            0x82 => self.res_0_d(),
            0x83 => self.res_0_e(),
            0x84 => self.res_0_h(),
            0x85 => self.res_0_l(),
            0x86 => self.res_0_at_hl(mem),
            0x87 => self.res_0_a(),
            0x88 => self.res_1_b(),
            0x89 => self.res_1_c(),
            0x8A => self.res_1_d(),
            0x8B => self.res_1_e(),
            0x8C => self.res_1_h(),
            0x8D => self.res_1_l(),
            0x8E => self.res_1_at_hl(mem),
            0x8F => self.res_1_a(),
            0x90 => self.res_2_b(),
            0x91 => self.res_2_c(),
            0x92 => self.res_2_d(),
            0x93 => self.res_2_e(),
            0x94 => self.res_2_h(),
            0x95 => self.res_2_l(),
            0x96 => self.res_2_at_hl(mem),
            0x97 => self.res_2_a(),
            0x98 => self.res_3_b(),
            0x99 => self.res_3_c(),
            0x9A => self.res_3_d(),
            0x9B => self.res_3_e(),
            0x9C => self.res_3_h(),
            0x9D => self.res_3_l(),
            0x9E => self.res_3_at_hl(mem),
            0x9F => self.res_3_a(),
            0xA0 => self.res_4_b(),
            0xA1 => self.res_4_c(),
            0xA2 => self.res_4_d(),
            0xA3 => self.res_4_e(),
            0xA4 => self.res_4_h(),
            0xA5 => self.res_4_l(),
            0xA6 => self.res_4_at_hl(mem),
            0xA7 => self.res_4_a(),
            0xA8 => self.res_5_b(),
            0xA9 => self.res_5_c(),
            0xAA => self.res_5_d(),
            0xAB => self.res_5_e(),
            0xAC => self.res_5_h(),
            0xAD => self.res_5_l(),
            0xAE => self.res_5_at_hl(mem),
            0xAF => self.res_5_a(),
            0xB0 => self.res_6_b(),
            0xB1 => self.res_6_c(),
            0xB2 => self.res_6_d(),
            0xB3 => self.res_6_e(),
            0xB4 => self.res_6_h(),
            0xB5 => self.res_6_l(),
            0xB6 => self.res_6_at_hl(mem),
            0xB7 => self.res_6_a(),
            0xB8 => self.res_7_b(),
            0xB9 => self.res_7_c(),
            0xBA => self.res_7_d(),
            0xBB => self.res_7_e(),
            0xBC => self.res_7_h(),
            0xBD => self.res_7_l(),
            0xBE => self.res_7_at_hl(mem),
            0xBF => self.res_7_a(),
            0xC0 => self.set_0_b(),
            0xC1 => self.set_0_c(),
            0xC2 => self.set_0_d(),
            0xC3 => self.set_0_e(),
            0xC4 => self.set_0_h(),
            0xC5 => self.set_0_l(),
            0xC6 => self.set_0_at_hl(mem),
            0xC7 => self.set_0_a(),
            0xC8 => self.set_1_b(),
            0xC9 => self.set_1_c(),
            0xCA => self.set_1_d(),
            0xCB => self.set_1_e(),
            0xCC => self.set_1_h(),
            0xCD => self.set_1_l(),
            0xCE => self.set_1_at_hl(mem),
            0xCF => self.set_1_a(),
            0xD0 => self.set_2_b(),
            0xD1 => self.set_2_c(),
            0xD2 => self.set_2_d(),
            0xD3 => self.set_2_e(),
            0xD4 => self.set_2_h(),
            0xD5 => self.set_2_l(),
            0xD6 => self.set_2_at_hl(mem),
            0xD7 => self.set_2_a(),
            0xD8 => self.set_3_b(),
            0xD9 => self.set_3_c(),
            0xDA => self.set_3_d(),
            0xDB => self.set_3_e(),
            0xDC => self.set_3_h(),
            0xDD => self.set_3_l(),
            0xDE => self.set_3_at_hl(mem),
            0xDF => self.set_3_a(),
            0xE0 => self.set_4_b(),
            0xE1 => self.set_4_c(),
            0xE2 => self.set_4_d(),
            0xE3 => self.set_4_e(),
            0xE4 => self.set_4_h(),
            0xE5 => self.set_4_l(),
            0xE6 => self.set_4_at_hl(mem),
            0xE7 => self.set_4_a(),
            0xE8 => self.set_5_b(),
            0xE9 => self.set_5_c(),
            0xEA => self.set_5_d(),
            0xEB => self.set_5_e(),
            0xEC => self.set_5_h(),
            0xED => self.set_5_l(),
            0xEE => self.set_5_at_hl(mem),
            0xEF => self.set_5_a(),
            0xF0 => self.set_6_b(),
            0xF1 => self.set_6_c(),
            0xF2 => self.set_6_d(),
            0xF3 => self.set_6_e(),
            0xF4 => self.set_6_h(),
            0xF5 => self.set_6_l(),
            0xF6 => self.set_6_at_hl(mem),
            0xF7 => self.set_6_a(),
            0xF8 => self.set_7_b(),
            0xF9 => self.set_7_c(),
            0xFA => self.set_7_d(),
            0xFB => self.set_7_e(),
            0xFC => self.set_7_h(),
            0xFD => self.set_7_l(),
            0xFE => self.set_7_at_hl(mem),
            0xFF => self.set_7_a(),
            _ => {
                println!("El opCode CB {:x} no está implementado", byte);
                self.halt = true;
            }
        }
        self.opcode_prefix = OpCodePrefix::None;;
    }
    fn exec_fd_cb_prefix(&mut self, mem: &mut Memory, byte: u8) {
        let mut addr = Z80::get_word(self.get_h(), self.get_l());
        addr += byte as u16;
        let op = mem.peek(addr);
        let op_code = self.read_bus(mem);
        match op_code {
            0x00 => self.rlc_to_b(op),
            0x01 => self.rlc_to_c(op),
            0x02 => self.rlc_to_d(op),
            0x03 => self.rlc_to_e(op),
            0x04 => self.rlc_to_h(op),
            0x05 => self.rlc_to_l(op),
            0x06 => self.rlc_at_ixy(mem, addr, op),
            0x07 => self.rlc_to_a(op),
            0x08 => self.rrc_to_b(op),
            0x09 => self.rrc_to_c(op),
            0x0A => self.rrc_to_d(op),
            0x0B => self.rrc_to_e(op),
            0x0C => self.rrc_to_h(op),
            0x0D => self.rrc_to_l(op),
            0x0E => self.rrc_at_ixy(mem, addr, op),
            0x0F => self.rrc_to_a(op),
            0x10 => self.rl_to_b(op),
            0x11 => self.rl_to_c(op),
            0x12 => self.rl_to_d(op),
            0x13 => self.rl_to_e(op),
            0x14 => self.rl_to_h(op),
            0x15 => self.rl_to_l(op),
            0x16 => self.rl_at_ixy(mem, addr, op),
            0x17 => self.rl_to_a(op),
            0x18 => self.rr_to_b(op),
            0x19 => self.rr_to_c(op),
            0x1A => self.rr_to_d(op),
            0x1B => self.rr_to_e(op),
            0x1C => self.rr_to_h(op),
            0x1D => self.rr_to_l(op),
            0x1E => self.rr_at_ixy(mem, addr, op),
            0x1F => self.rr_to_a(op),
            0x20 => self.sla_to_b(op),
            0x21 => self.sla_to_c(op),
            0x22 => self.sla_to_d(op),
            0x23 => self.sla_to_e(op),
            0x24 => self.sla_to_h(op),
            0x25 => self.sla_to_l(op),
            0x26 => self.sla_at_ixy(mem, addr, op),
            0x27 => self.sla_to_a(op),
            0x28 => self.sra_to_b(op),
            0x29 => self.sra_to_c(op),
            0x2A => self.sra_to_d(op),
            0x2B => self.sra_to_e(op),
            0x2C => self.sra_to_h(op),
            0x2D => self.sra_to_l(op),
            0x2E => self.sra_at_ixy(mem, addr, op),
            0x2F => self.sra_to_a(op),
            0x30 => self.sll_to_b(op),
            0x31 => self.sll_to_c(op),
            0x32 => self.sll_to_d(op),
            0x33 => self.sll_to_e(op),
            0x34 => self.sll_to_h(op),
            0x35 => self.sll_to_l(op),
            0x36 => self.sll_at_ixy(mem, addr, op),
            0x37 => self.sll_to_a(op),
            0x38 => self.srl_to_b(op),
            0x39 => self.srl_to_c(op),
            0x3A => self.srl_to_d(op),
            0x3B => self.srl_to_e(op),
            0x3C => self.srl_to_h(op),
            0x3D => self.srl_to_l(op),
            0x3E => self.srl_at_ixy(mem, addr, op),
            0x3F => self.srl_to_a(op),
            0x40 | 0x41 | 0x42 | 0x43 | 0x44 | 0x45 | 0x46 | 0x47 => self.bit_0_ixy(op),
            0x48 | 0x49 | 0x4A | 0x4B | 0x4C | 0x4D | 0x4E | 0x4F => self.bit_1_ixy(op),
            0x50 | 0x51 | 0x52 | 0x53 | 0x54 | 0x55 | 0x56 | 0x57 => self.bit_2_ixy(op),
            0x58 | 0x59 | 0x5A | 0x5B | 0x5C | 0x5D | 0x5E | 0x5F => self.bit_3_ixy(op),
            0x60 | 0x61 | 0x62 | 0x63 | 0x64 | 0x65 | 0x66 | 0x67 => self.bit_4_ixy(op),
            0x68 | 0x69 | 0x6A | 0x6B | 0x6C | 0x6D | 0x6E | 0x6F => self.bit_5_ixy(op),
            0x70 | 0x71 | 0x72 | 0x73 | 0x74 | 0x75 | 0x76 | 0x77 => self.bit_6_ixy(op),
            0x78 | 0x79 | 0x7A | 0x7B | 0x7C | 0x7D | 0x7E | 0x7F => self.bit_7_ixy(op),
            0x80 | 0x81 | 0x82 | 0x83 | 0x84 | 0x85 | 0x86 | 0x87 => self.res_0_ixy(mem, addr, op),
            0x88 | 0x89 | 0x8A | 0x8B | 0x8C | 0x8D | 0x8E | 0x8F => self.res_1_ixy(mem, addr, op),
            0x90 | 0x91 | 0x92 | 0x93 | 0x94 | 0x95 | 0x96 | 0x97 => self.res_2_ixy(mem, addr, op),
            0x98 | 0x99 | 0x9A | 0x9B | 0x9C | 0x9D | 0x9E | 0x9F => self.res_3_ixy(mem, addr, op),
            0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5 | 0xA6 | 0xA7 => self.res_4_ixy(mem, addr, op),
            0xA8 | 0xA9 | 0xAA | 0xAB | 0xAC | 0xAD | 0xAE | 0xAF => self.res_5_ixy(mem, addr, op),
            0xB0 | 0xB1 | 0xB2 | 0xB3 | 0xB4 | 0xB5 | 0xB6 | 0xB7 => self.res_6_ixy(mem, addr, op),
            0xB8 | 0xB9 | 0xBA | 0xBB | 0xBC | 0xBD | 0xBE | 0xBF => self.res_7_ixy(mem, addr, op),
            0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC4 | 0xC5 | 0xC6 | 0xC7 => self.set_0_ixy(mem, addr, op),
            0xC8 | 0xC9 | 0xCA | 0xCB | 0xCC | 0xCD | 0xCE | 0xCF => self.set_1_ixy(mem, addr, op),
            0xD0 | 0xD1 | 0xD2 | 0xD3 | 0xD4 | 0xD5 | 0xD6 | 0xD7 => self.set_2_ixy(mem, addr, op),
            0xD8 | 0xD9 | 0xDA | 0xDB | 0xDC | 0xDD | 0xDE | 0xDF => self.set_3_ixy(mem, addr, op),
            0xE0 | 0xE1 | 0xE2 | 0xE3 | 0xE4 | 0xE5 | 0xE6 | 0xE7 => self.set_4_ixy(mem, addr, op),
            0xE8 | 0xE9 | 0xEA | 0xEB | 0xEC | 0xED | 0xEE | 0xEF => self.set_5_ixy(mem, addr, op),
            0xF0 | 0xF1 | 0xF2 | 0xF3 | 0xF4 | 0xF5 | 0xF6 | 0xF7 => self.set_6_ixy(mem, addr, op),
            0xF8 | 0xF9 | 0xFA | 0xFB | 0xFC | 0xFD | 0xFE | 0xFF => self.set_7_ixy(mem, addr, op),
            _ => {
                let prefix = match self.opcode_prefix {
                    OpCodePrefix::FdCb => "FD CB",
                    OpCodePrefix::DdCb => "DD CB",
                    _ => "Otro",
                };
                println!("El opCode {} {:x} no está implementado", prefix, byte);
                self.halt = true;
            }
        };
        self.opcode_prefix = OpCodePrefix::None;
    }
    fn exec_ed_prefix(&mut self, mem: &mut Memory, byte: u8) {
        let mut new_prefix = OpCodePrefix::None;
        match byte {
            //0x40 => in_b_at_c(mem),
            //0x41 => out_at_c_b(mem),
            0x42 => self.sbc_hl_bc(),
            0x43 => self.ld_at_nn_bc(mem),
            0x44 => self.neg(),
            0x45 => self.retn(mem),
            //0x46 => im_0(),
            0x47 => self.ld_i_a(),
            //0x48 => in_c_at_c(mem),
            //0x49 => out_c_at_c(mem),
            0x4A => self.adc_hl_bc(),
            0x4B => self.ld_bc_at_nn(mem),
            0x4C => self.neg(),
            0x4F => self.ld_r_a(),
            0x52 => self.sbc_hl_de(),
            0x53 => self.ld_nn_de(mem),
            0x54 => self.neg(),
            0x55 => self.retn(mem),
            0x57 => self.ld_a_i(),
            0x5A => self.adc_hl_de(),
            0x5B => self.ld_de_at_nn(mem),
            0x5C => self.neg(),
            0x5F => self.ld_a_r(),
            0x62 => self.sbc_hl_hl(),
            0x63 => self.ld_nn_hl(mem),
            0x64 => self.neg(),
            0x65 => self.retn(mem),
            0x67 => self.rrd(),
            0x6A => self.adc_hl_hl(),
            0x6B => self.ld_hl_at_nn(mem),
            0x6C => self.neg(),
            //0x6F => self.rld(),
            0x72 => self.sbc_hl_sp(),
            0x73 => self.ld_nn_sp(mem),
            0x74 => self.neg(),
            0x75 => self.retn(mem),
            0x7A => self.adc_hl_sp(),
            0x7B => self.ld_sp_at_nn(mem),
            0x7C => self.neg(),
            0xB8 => self.lddr(mem),
            0xCB => new_prefix = OpCodePrefix::CB,
            0xDD => new_prefix = OpCodePrefix::DD,
            0xED => new_prefix = OpCodePrefix::ED,
            0xFD => new_prefix = OpCodePrefix::FD,
            _ => {
                println!("El opCode ED {:x} no está implementado", byte);
                self.halt = true;
            }
        }
        self.opcode_prefix = new_prefix;
    }
    fn read_bus(&mut self, mem: &Memory) -> u8 {
        let res = mem.peek(self.pc);
        self.pc += 1;
        res
    }
    fn nop(&mut self) {
        self.save_op("NOP");
    }
    fn di(&mut self) {
        self.save_op("DI");
        self.iff1 = false;
        self.iff2 = false;
    }
    fn halt(&mut self) {
        self.halted = true;
        self.save_op("HALT");
    }
    fn xor_r(&mut self, value: u8) {
        self.a = self.a ^ value;
        let mut cond = (self.a as i8) < 0;
        self.set_reset_flag(cond, S);
        cond = self.a == 0;
        self.set_reset_flag(cond, Z);
        cond = (self.a & 0xF) < (self.h & 0xF);
        self.set_reset_flag(cond, H);
        cond = Z80::check_byte_parity(self.a);
        self.set_reset_flag(cond, P_V);
        self.reset_flag(N);
        self.reset_flag(C);
    }
    fn xor_b(&mut self) {
        let op = self.b;
        self.xor_r(op);
        self.save_op("XOR B");
    }
    fn xor_c(&mut self) {
        let op = self.c;
        self.xor_r(op);
        self.save_op("XOR C");
    }
    fn xor_d(&mut self) {
        let op = self.d;
        self.xor_r(op);
        self.save_op("XOR D");
    }
    fn xor_e(&mut self) {
        let op = self.e;
        self.xor_r(op);
        self.save_op("XOR E");
    }
    fn xor_h(&mut self) {
        let op = self.get_h();
        self.xor_r(op);
        self.save_op("XOR H");
    }
    fn xor_l(&mut self) {
        let op = self.get_l();
        self.xor_r(op);
        self.save_op("XOR L");
    }
    fn xor_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let op = mem.peek(addr);
        self.xor_r(op);
        self.save_op("XOR (HL)");
    }
    fn xor_a(&mut self) {
        let op = self.a;
        self.xor_r(op);
        self.save_op("XOR A");
    }
    fn xor_n(&mut self, mem: &Memory) {
        let op = self.read_bus(mem);
        self.xor_r(op);
        let msg = format!("XOR {:x}", op);
        self.save_op(&msg);
    }
    fn or_r(&mut self, value: u8) {
        self.a = self.a | value;
        let mut cond = (self.a as i8) < 0;
        self.set_reset_flag(cond, S);
        cond = self.a == 0;
        self.set_reset_flag(cond, Z);
        self.reset_flag(H);
        // TODO: Mirar cómo funciona este flag
        self.reset_flag(P_V);
        self.reset_flag(N);
        self.reset_flag(C);
    }
    fn or_b(&mut self) {
        let op = self.b;
        self.or_r(op);
        self.save_op("OR B");
    }
    fn or_c(&mut self) {
        let op = self.c;
        self.or_r(op);
        self.save_op("OR C");
    }
    fn or_d(&mut self) {
        let op = self.d;
        self.or_r(op);
        self.save_op("OR D");
    }
    fn or_e(&mut self) {
        let op = self.e;
        self.or_r(op);
        self.save_op("OR E");
    }
    fn or_h(&mut self) {
        let op = self.get_h();
        self.or_r(op);
        self.save_op("OR H");
    }
    fn or_l(&mut self) {
        let op = self.get_l();
        self.or_r(op);
        self.save_op("OR L");
    }
    fn or_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let op = mem.peek(addr);
        self.or_r(op);
        self.save_op("OR (HL)");
    }
    fn or_a(&mut self) {
        let op = self.a;
        self.or_r(op);
        self.save_op("XOR A");
    }
    fn or_n(&mut self, mem: &Memory) {
        let op = self.read_bus(mem);
        self.or_r(op);
        let msg = format!("OR {:x}", op);
        self.save_op(&msg);
    }
    fn ld_bc(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        self.c = x1;
        self.b = x2;
        let dir = Z80::get_word(x2, x1);
        let msg = format!("LD BC {:x}", dir);
        self.save_op(&msg);
    }
    fn ld_at_bc_a(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.b, self.c);
        mem.poke(addr, self.a);
        self.save_op("LD (BC) A");
    }
    fn ld_at_de_a(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.d, self.e);
        mem.poke(addr, self.a);
        self.save_op("LD (DE) A");
    }
    fn ld_at_hl_b(&mut self, mem: &mut Memory) {
        let addr = self.get_indirect_hl(mem);
        mem.poke(addr, self.b);
        self.save_op("LD (HL) B");
    }
    fn ld_at_hl_c(&mut self, mem: &mut Memory) {
        let addr = self.get_indirect_hl(mem);
        mem.poke(addr, self.c);
        self.save_op("LD (HL) C");
    }
    fn ld_at_hl_d(&mut self, mem: &mut Memory) {
        let addr = self.get_indirect_hl(mem);
        mem.poke(addr, self.d);
        self.save_op("LD (HL) D");
    }
    fn ld_at_hl_e(&mut self, mem: &mut Memory) {
        let addr = self.get_indirect_hl(mem);
        mem.poke(addr, self.e);
        self.save_op("LD (HL) E");
    }
    fn ld_at_hl_h(&mut self, mem: &mut Memory) {
        let addr = self.get_indirect_hl(mem);
        mem.poke(addr, self.h);
        self.save_op("LD (HL) H");
    }
    fn ld_at_hl_l(&mut self, mem: &mut Memory) {
        let addr = self.get_indirect_hl(mem);
        mem.poke(addr, self.l);
        self.save_op("LD (HL) L");
    }
    fn ld_at_hl_a(&mut self, mem: &mut Memory) {
        let addr = self.get_indirect_hl(mem);
        mem.poke(addr, self.a);
        self.save_op("LD (HL) A");
    }
    fn ld_sp_hl(&mut self) {
        let new_sp = Z80::get_word(self.h, self.l);
        self.sp = new_sp;
        self.save_op("LD SP HL");
    }
    fn rlca(&mut self) {
        let bit = self.a & 0x80;
        self.a = (self.a << 1) | bit;
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag(bit != 0, C);
        self.save_op("RLCA");
    }
    fn rla(&mut self) {
        let bit = self.a & 0x80;
        let old_c = self.f & C;
        self.a = (self.a << 1) | old_c;
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag(bit != 0, C);
        self.set_reset_flag(bit != 0, C);
        self.save_op("RLA");
    }
    fn rrca(&mut self) {
        let bit = self.a & 0x01;
        self.a = (self.a >> 1) | (bit << 7);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag(bit != 0, C);
        self.set_reset_flag(bit != 0, C);
        self.save_op("RRCA");
    }
    fn rra(&mut self) {
        let bit = self.a & 0x01;
        let old_c = self.f & C;
        self.a = (self.a >> 1) | (bit << old_c);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag(bit != 0, C);
        self.set_reset_flag(bit != 0, C);
        self.save_op("RRA");
    }
    fn ex_af_af_alt(&mut self) {
        let swap_hi = self.a;
        let swap_lo = self.f;
        self.a = self.a_alt;
        self.f = self.f_alt;
        self.a_alt = swap_hi;
        self.f_alt = swap_lo;
        self.save_op("EX AF A'F'");
    }
    fn ex_de_hl(&mut self) {
        let old_d = self.d;
        self.d = self.h;
        self.h = old_d;

        let old_e = self.e;
        self.e = self.l;
        self.l = old_e;
        self.save_op("EX DE HL");
    }
    fn add_hl_ss(&mut self, op: u32) {
        let hl: u32 = self.get_hl() as u32;
        let sum = hl + op;
        self.set_l((sum & 0xff) as u8);
        self.set_h(((sum & 0xffff) >> 8) as u8);

        self.reset_flag(N);
        self.set_reset_flag(sum > 0xffff, C);
        self.set_reset_flag((hl & 0x7ff) + (op & 0x7ff) < (sum & 0x7ff), H);
    }
    fn add_hl_bc(&mut self) {
        let bc = Z80::get_word(self.b, self.c) as u32;
        self.add_hl_ss(bc);
        self.save_op("ADD HL BC");
    }
    fn add_hl_de(&mut self) {
        let de = Z80::get_word(self.d, self.e) as u32;
        self.add_hl_ss(de);
        self.save_op("ADD HL DE");
    }
    fn add_hl_hl(&mut self) {
        let hl: u32 = self.get_hl() as u32;
        self.add_hl_ss(hl);
        self.save_op("ADD HL HL");
    }
    fn add_hl_sp(&mut self) {
        let sp = self.sp as u32;
        self.add_hl_ss(sp);
        self.save_op("ADD HL SP");
    }
    fn add_a_r(&mut self, other: u8) {
        let sum = (self.a as i16) + (other as i16);
        self.a = (sum & 0xff) as u8;
        self.set_reset_flag(((sum & 0xff) as i8) < 0, S);
        self.set_reset_flag(sum == 0, Z);
        self.set_reset_flag((sum - 1) & 0xf == 0xf, H);
        self.set_reset_flag(sum > (sum & 0xff), P_V);
        self.reset_flag(N);
        self.set_reset_flag((sum - 1) & 0xff == 0xff, C);
    }
    fn add_a_b(&mut self) {
        let op = self.b;
        self.add_a_r(op);
        self.save_op("ADD A B");
    }
    fn add_a_c(&mut self) {
        let op = self.c;
        self.add_a_r(op);
        self.save_op("ADD A C");
    }
    fn add_a_d(&mut self) {
        let op = self.d;
        self.add_a_r(op);
        self.save_op("ADD A D");
    }
    fn add_a_e(&mut self) {
        let op = self.e;
        self.add_a_r(op);
        self.save_op("ADD A E");
    }
    fn add_a_h(&mut self) {
        let op = self.get_h();
        self.add_a_r(op);
        self.save_op("ADD A H");
    }
    fn add_a_l(&mut self) {
        let op = self.get_l();
        self.add_a_r(op);
        self.save_op("ADD A L");
    }
    fn add_a_a(&mut self) {
        let op = self.a;
        self.add_a_r(op);
        self.save_op("ADD A A");
    }
    fn add_a_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let val = mem.peek(addr);
        self.add_a_r(val);
        self.save_op("ADD A (HL)");
    }
    fn adc_a_r(&mut self, other: u8) {
        let c = (self.f & C) as i16;
        let sum = self.a as i16 + other as i16 + c;
        self.a = (sum & 0xff) as u8;
        self.set_reset_flag(sum < 0, S);
        self.set_reset_flag(sum == 0, Z);
        self.set_reset_flag((sum - 1) & 0xf == 0xf, H);
        self.set_reset_flag(sum > (sum & 0xff), P_V);
        self.reset_flag(N);
        self.set_reset_flag((sum - 1) & 0xff == 0xff, C);
    }
    fn adc_a_b(&mut self) {
        let op = self.b;
        self.adc_a_r(op);
        self.save_op("ADC A B");
    }
    fn adc_a_c(&mut self) {
        let op = self.c;
        self.adc_a_r(op);
        self.save_op("ADC A C");
    }
    fn adc_a_d(&mut self) {
        let op = self.d;
        self.adc_a_r(op);
        self.save_op("ADC A D");
    }
    fn adc_a_e(&mut self) {
        let op = self.e;
        self.adc_a_r(op);
        self.save_op("ADC A E");
    }
    fn adc_a_h(&mut self) {
        let op = self.get_h();
        self.adc_a_r(op);
        self.save_op("ADC A H");
    }
    fn adc_a_l(&mut self) {
        let op = self.get_l();
        self.adc_a_r(op);
        self.save_op("ADC A L");
    }
    fn adc_a_a(&mut self) {
        let op = self.a;
        self.adc_a_r(op);
        self.save_op("ADC A A");
    }
    fn adc_a_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let val = mem.peek(addr);
        self.adc_a_r(val);
        self.save_op("ADC A (HL)");
    }
    fn adc_a_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.adc_a_r(x1);
        let msg = format!("ADC A {:x}", x1);
        self.save_op(&msg);
    }
    fn sub_a_r(&mut self, other: u8) {
        let diff = self.a as i16 - other as i16;
        let old_a = self.a;
        self.a = diff as u8;
        self.set_reset_flag(diff < 0, S);
        self.set_reset_flag(diff == 0, Z);
        self.set_reset_flag((old_a & 0xf) < (other & 0xf), H);
        let overflow_high = (old_a >= 0x80) & (other >= 0x80) & ((diff as i8) > 0);
        let overflow_low = (old_a < 0x80) & (other < 0x80) & ((diff as i8) < 0);
        self.set_reset_flag(overflow_high | overflow_low, P_V);
        self.set_flag(N);
        self.set_reset_flag((diff & 0x80) > 0, C);
    }
    fn sub_a_b(&mut self) {
        let op = self.b;
        self.sub_a_r(op);
        self.save_op("SUB A B");
    }
    fn sub_a_c(&mut self) {
        let op = self.c;
        self.sub_a_r(op);
        self.save_op("SUB A C");
    }
    fn sub_a_d(&mut self) {
        let op = self.d;
        self.sub_a_r(op);
        self.save_op("SUB A D");
    }
    fn sub_a_e(&mut self) {
        let op = self.e;
        self.sub_a_r(op);
        self.save_op("SUB A E");
    }
    fn sub_a_h(&mut self) {
        let op = self.get_h();
        self.sub_a_r(op);
        self.save_op("SUB A H");
    }
    fn sub_a_l(&mut self) {
        let op = self.get_l();
        self.sub_a_r(op);
        self.save_op("SUB A L");
    }
    fn sub_a_a(&mut self) {
        let op = self.a;
        self.sub_a_r(op);
        self.save_op("SUB A A");
    }
    fn sub_a_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let val = mem.peek(addr);
        self.sub_a_r(val);
        self.save_op("SUB A (HL)");
    }
    fn sub_a_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.sub_a_r(x1);
        let msg = format!("SUB A {:x}", x1);
        self.save_op(&msg);
    }
    fn sbc_a_r(&mut self, other: u8) {
        let c = self.f & 0x01;
        let diff = self.a as i16 - other as i16 - c as i16;
        let old_a = self.a;
        self.a = diff as u8;
        self.set_reset_flag(diff < 0, S);
        self.set_reset_flag(diff == 0, Z);
        self.set_reset_flag((old_a & 0xf) < (other & 0xf), H);
        let overflow_high = (old_a >= 0x80) & (other >= 0x80) & ((diff as i8) > 0);
        let overflow_low = (old_a < 0x80) & (other < 0x80) & ((diff as i8) < 0);
        self.set_reset_flag(overflow_high | overflow_low, P_V);
        self.set_flag(N);
        self.set_reset_flag((diff & 0x80) > 0, C);
    }
    fn sbc_a_b(&mut self) {
        let op = self.b;
        self.sbc_a_r(op);
        self.save_op("SBC A B");
    }
    fn sbc_a_c(&mut self) {
        let op = self.c;
        self.sbc_a_r(op);
        self.save_op("SBC A C");
    }
    fn sbc_a_d(&mut self) {
        let op = self.d;
        self.sbc_a_r(op);
        self.save_op("SBC A D");
    }
    fn sbc_a_e(&mut self) {
        let op = self.e;
        self.sbc_a_r(op);
        self.save_op("SBC A E");
    }
    fn sbc_a_h(&mut self) {
        let op = self.get_h();
        self.sbc_a_r(op);
        self.save_op("SBC A H");
    }
    fn sbc_a_l(&mut self) {
        let op = self.get_l();
        self.sbc_a_r(op);
        self.save_op("SBC A L");
    }
    fn sbc_a_a(&mut self) {
        let op = self.a;
        self.sbc_a_r(op);
        self.save_op("SBC A A");
    }
    fn sbc_a_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let val = mem.peek(addr);
        self.sbc_a_r(val);
        self.save_op("SBC A (HL)");
    }
    fn sbc_a_n(&mut self, mem: &Memory) {
        let n = self.read_bus(mem);
        self.sbc_a_r(n);
        let msg = format!("SBC A {:x}", n);
        self.save_op(&msg);
    }
    fn ld_a_b(&mut self) {
        self.a = self.b;
        self.save_op("LD A B");
    }
    fn ld_a_c(&mut self) {
        self.a = self.c;
        self.save_op("LD A C")
    }
    fn ld_a_d(&mut self) {
        self.a = self.d;
        self.save_op("LD A D")
    }
    fn ld_a_e(&mut self) {
        self.a = self.e;
        self.save_op("LD A E")
    }
    fn ld_a_h(&mut self) {
        self.a = self.get_h();
        self.save_op("LD A H")
    }
    fn ld_a_l(&mut self) {
        self.a = self.get_l();
        self.save_op("LD A L")
    }
    fn ld_a_a(&mut self) {
        self.a = self.a;
        self.save_op("LD A A")
    }
    fn ld_a_at_bc(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.b, self.c);
        let value = mem.peek(addr);
        self.a = value;
        self.save_op("LD A (BC)");
    }
    fn ld_a_at_de(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.d, self.e);
        let value = mem.peek(addr);
        self.a = value;
        self.save_op("LD A (DE)");
    }
    fn ld_a_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let value = mem.peek(addr);
        self.a = value;
        self.save_op("LD A (HL)");
    }
    fn ld_a_at_nn(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        let addr = Z80::get_word(x2, x1);
        self.a = mem.peek(addr);
        let msg = format!("LD A ({:x})", addr);
        self.save_op(&msg);
    }
    fn inc_b(&mut self) {
        let inc: u16 = self.b as u16 + 1;
        let init = self.b;
        self.adjust_f_inc_n(init, inc);
        self.b = (inc & 0xff) as u8;
        self.save_op("INC B");
    }
    fn inc_bc(&mut self) {
        let mut bc = Z80::get_word(self.b, self.c);
        bc = bc.wrapping_add(1);
        self.c = (bc & 0xff) as u8;
        self.b = ((bc >> 8) & 0xff) as u8;
        self.save_op("INC BC");
    }
    fn inc_de(&mut self) {
        let mut de = Z80::get_word(self.d, self.e);
        de = de.wrapping_add(1);
        self.d = (de & 0xff) as u8;
        self.e = ((de >> 8) & 0xff) as u8;
        self.save_op("DEC DE");
    }
    fn inc_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let val = mem.peek(addr) + 1;
        mem.poke(addr, val);
        self.set_reset_flag((val as i8) < 0, S);
        self.set_reset_flag(val == 0, Z);
        self.set_reset_flag((val - 1) & 0xf == 0xf, H);
        self.set_reset_flag((val - 1) == 0x7f, P_V);
        self.reset_flag(N);
        self.save_op("INC (HL)");
    }
    fn dec_bc(&mut self) {
        let mut bc = Z80::get_word(self.b, self.c);
        bc = bc.wrapping_sub(1);
        self.c = (bc & 0xff) as u8;
        self.b = ((bc >> 8) & 0xff) as u8;
        self.save_op("DEC BC");
    }
    fn dec_de(&mut self) {
        let mut de = Z80::get_word(self.d, self.e);
        de = de.wrapping_sub(1);
        self.d = (de & 0xff) as u8;
        self.e = ((de >> 8) & 0xff) as u8;
        self.save_op("DEC DE");
    }
    fn dec_hl(&mut self) {
        let mut hl = self.get_hl();
        hl = hl.wrapping_sub(1);
        let hi = ((hl >> 8) & 0xff) as u8;
        let lo = (hl & 0xff) as u8;
        self.set_h(hi);
        self.set_l(lo);
        self.save_op("DEC HL");
    }
    fn inc_sp(&mut self) {
        self.sp = self.sp.wrapping_add(1);
        self.save_op("INC SP");
    }
    fn dec_sp(&mut self) {
        self.sp = self.sp.wrapping_sub(1);
        self.save_op("DEC SP");
    }
    fn daa(&mut self) {
        let c = (self.f & C) == C;
        let h = (self.f & H) == H;
        if ((self.a & 0x0f) > 0x09) | h {
            self.a += 0x06;
        }
        if ((self.a & 0xf0) > 0x90) | c {
            self.a += 0x60;
        }
        self.save_op("DAA");
    }
    fn cpl(&mut self) {
        self.a = !self.a;
        self.save_op("CPL");
    }
    fn inc_c(&mut self) {
        let inc: u16 = self.c as u16 + 1;
        let init = self.c;
        self.adjust_f_inc_n(init, inc);
        self.c = (inc & 0xff) as u8;
        self.save_op("INC C");
    }
    fn inc_d(&mut self) {
        let inc: u16 = self.d as u16 + 1;
        let init = self.d;
        self.adjust_f_inc_n(init, inc);
        self.d = (inc & 0xff) as u8;
        self.save_op("INC D");
    }
    fn inc_e(&mut self) {
        let inc: u16 = self.e as u16 + 1;
        let init = self.e;
        self.adjust_f_inc_n(init, inc);
        self.e = (inc & 0xff) as u8;
        self.save_op("INC E");
    }
    fn inc_h(&mut self) {
        let inc: u16 = self.get_h() as u16 + 1;
        let init = self.h;
        self.adjust_f_inc_n(init, inc);
        self.set_h((inc & 0xff) as u8);
        self.save_op("INC H");
    }
    fn inc_l(&mut self) {
        let init = self.get_l();
        let inc: u16 = init as u16 + 1;
        self.adjust_f_inc_n(init, inc);
        self.set_l((inc & 0xff) as u8);
        self.save_op("INC C");
    }
    fn inc_a(&mut self) {
        let inc: u16 = self.a as u16 + 1;
        let init = self.a;
        self.adjust_f_inc_n(init, inc);
        self.a = (inc & 0xff) as u8;
        self.save_op("INC A");
    }
    fn dec_b(&mut self) {
        let inc: u16 = self.b as u16 - 1;
        let init = self.b;
        self.adjust_f_dec_n(init, inc);
        self.b = (inc & 0xff) as u8;
        self.save_op("DEC B");
    }
    fn dec_c(&mut self) {
        let inc: u16 = self.c as u16 - 1;
        let init = self.c;
        self.adjust_f_dec_n(init, inc);
        self.c = (inc & 0xff) as u8;
        self.save_op("DEC C");
    }
    fn dec_d(&mut self) {
        let inc: u16 = self.d as u16 - 1;
        let init = self.d;
        self.adjust_f_dec_n(init, inc);
        self.d = (inc & 0xff) as u8;
        self.save_op("DEC D");
    }
    fn dec_e(&mut self) {
        let inc: u16 = self.e as u16 - 1;
        let init = self.e;
        self.adjust_f_dec_n(init, inc);
        self.e = (inc & 0xff) as u8;
        self.save_op("DEC E");
    }
    fn dec_h(&mut self) {
        let inc: u16 = self.get_h() as u16 - 1;
        let init = self.h;
        self.adjust_f_dec_n(init, inc);
        self.set_h((inc & 0xff) as u8);
        self.save_op("DEC H");
    }
    fn dec_l(&mut self) {
        let init = self.get_l();
        let inc: u16 = init as u16 - 1;
        self.adjust_f_dec_n(init, inc);
        self.set_l((inc & 0xff) as u8);
        self.save_op("DEC L");
    }
    fn dec_a(&mut self) {
        let inc: u16 = self.a as u16 - 1;
        let init = self.a;
        self.adjust_f_dec_n(init, inc);
        self.a = (inc & 0xff) as u8;
        self.save_op("DEC A");
    }
    fn adjust_f_inc_n(&mut self, initial: u8, result: u16) {
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(initial == 0xff, H);
        self.set_reset_flag(initial == 0x7f, P_V);
        self.reset_flag(N);
    }
    fn adjust_f_dec_n(&mut self, initial: u8, result: u16) {
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag((initial & 0xf) < (result as u8 & 0xf), H);
        self.set_reset_flag(initial == 0x80, P_V);
        self.reset_flag(N);
    }
    fn ld_de(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        self.d = x2;
        self.e = x1;
        let dir = Z80::get_word(x2, x1);
        let msg = format!("LD DE {:x}", dir);
        self.save_op(&msg);
    }

    fn jr_nz_e(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem) as i8;
        let x2 = self.pc as i16 + x1 as i16;
        if !Z80::check_flag(self.f, Z) {
            self.pc = x2 as u16;
        }
        let msg = format!("JR NZ {:x}", x1);
        self.save_op(&msg);
    }
    fn jr_e(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem) as i8;
        let x2 = self.pc as i16 + x1 as i16;
        self.pc = x2 as u16;
        let msg = format!("JR {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_hl(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        self.set_h(x2);
        self.set_l(x1);

        let dir = Z80::get_word(x2, x1);
        let msg = format!("LD HL {:x}", dir);
        self.save_op(&msg);
    }
    fn inc_hl(&mut self) {
        let mut hl: u32 = Z80::get_word(self.get_h(), self.get_l()) as u32;
        hl += 1;
        let hi = ((hl >> 8) & 0xff) as u8;
        let lo = (hl & 0xff) as u8;
        self.set_h(hi);
        self.set_l(lo);
        self.save_op("INC HL")
    }
    fn ld_sp(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        let dir = Z80::get_word(x1, x2);
        self.sp = dir;
        let msg = format!("LD SP {:x} {:x}", x1, x2);
        self.save_op(&msg);
    }
    fn dec_at_hl(&mut self, mem: &mut Memory) {
        let dir = Z80::get_word(self.get_h(), self.get_l());
        let value = mem.peek(dir) as i8;
        let res = (value as i16) - 1;
        let to_mem = (res & 0xff) as u8;
        mem.poke(dir, to_mem);
        let mut cond = res < 0;
        self.set_reset_flag(cond, S);
        cond = res == 0;
        self.set_reset_flag(cond, Z);
        cond = (value & 0xf) < ((res & 0xf) as i8);
        self.set_reset_flag(cond, H);
        cond = value as u8 == 0x80;
        self.set_reset_flag(cond, P_V);
        self.set_flag(N);
        self.save_op("DEC (HL)");
    }
    fn ld_hl_n(&mut self, mem: &mut Memory) {
        let address = Z80::get_word(self.get_h(), self.get_l());
        let n = self.read_bus(mem);
        mem.poke(address, n);
        let msg = format!("LD HL {:x}", n);
        self.save_op(&msg);
    }
    fn jp_nn(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        let dir = ((x2 as u16) << 8) + (x1 as u16);

        self.pc = dir;
        let msg = format!("JP {:x} {:x}", x2, x1);
        self.save_op(&msg);
    }
    fn ld_b_b(&mut self) {
        self.b = self.b;
        self.save_op("LD B B");
    }
    fn ld_b_c(&mut self) {
        self.b = self.c;
        self.save_op("LD B C");
    }
    fn ld_b_d(&mut self) {
        self.b = self.d;
        self.save_op("LD B D");
    }
    fn ld_b_e(&mut self) {
        self.b = self.e;
        self.save_op("LD B E");
    }
    fn ld_b_h(&mut self) {
        self.b = self.get_h();
        self.save_op("LD B H");
    }
    fn ld_b_l(&mut self) {
        self.b = self.get_l();
        self.save_op("LD B L");
    }
    fn ld_b_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let byte = mem.peek(addr);
        self.b = byte;
        self.save_op("LD B HL");
    }
    fn ld_b_a(&mut self) {
        self.b = self.a;
        self.save_op("LD B A");
    }
    fn ld_c_b(&mut self) {
        self.c = self.b;
        self.save_op("LD C B");
    }
    fn ld_c_c(&mut self) {
        self.c = self.c;
        self.save_op("LD C C");
    }
    fn ld_c_d(&mut self) {
        self.c = self.d;
        self.save_op("LD C D");
    }
    fn ld_c_e(&mut self) {
        self.c = self.e;
        self.save_op("LD C E");
    }
    fn ld_c_h(&mut self) {
        self.c = self.get_h();
        self.save_op("LD C H");
    }
    fn ld_c_l(&mut self) {
        self.c = self.get_l();
        self.save_op("LD C L");
    }
    fn ld_c_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let byte = mem.peek(addr);
        self.c = byte;
        self.save_op("LD C HL");
    }
    fn ld_c_a(&mut self) {
        self.c = self.a;
        self.save_op("LD C A");
    }
    fn ld_d_b(&mut self) {
        self.d = self.b;
        self.save_op("LD D B");
    }

    fn ld_d_c(&mut self) {
        self.d = self.c;
        self.save_op("LD D C");
    }

    fn ld_d_d(&mut self) {
        self.d = self.d;
        self.save_op("LD D D");
    }
    fn ld_d_e(&mut self) {
        self.d = self.e;
        self.save_op("LD D E");
    }
    fn ld_d_h(&mut self) {
        self.d = self.get_h();
        self.save_op("LD D H");
    }
    fn ld_d_l(&mut self) {
        self.d = self.get_l();
        self.save_op("LD D L");
    }
    fn ld_d_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let byte = mem.peek(addr);
        self.d = byte;
        self.save_op("LD D HL");
    }
    fn ld_d_a(&mut self) {
        self.d = self.a;
        self.save_op("LD D A");
    }
    fn ld_e_b(&mut self) {
        self.e = self.b;
        self.save_op("LD E B");
    }

    fn ld_e_c(&mut self) {
        self.e = self.c;
        self.save_op("LD E C");
    }

    fn ld_e_d(&mut self) {
        self.e = self.d;
        self.save_op("LD E D");
    }
    fn ld_e_e(&mut self) {
        self.e = self.e;
        self.save_op("LD E E");
    }
    fn ld_e_h(&mut self) {
        self.e = self.get_h();
        self.save_op("LD E H");
    }
    fn ld_e_l(&mut self) {
        self.e = self.get_l();
        self.save_op("LD E L");
    }
    fn ld_e_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let byte = mem.peek(addr);
        self.e = byte;
        self.save_op("LD E HL");
    }
    fn ld_e_a(&mut self) {
        self.e = self.a;
        self.save_op("LD E A");
    }

    fn ld_h_b(&mut self) {
        let op = self.b;
        self.set_h(op);
        self.save_op("LD H B");
    }

    fn ld_h_c(&mut self) {
        let op = self.c;
        self.set_h(op);
        self.save_op("LD H C");
    }

    fn ld_h_d(&mut self) {
        let op = self.d;
        self.set_h(op);
        self.save_op("LD H D");
    }
    fn ld_h_e(&mut self) {
        let op = self.e;
        self.set_h(op);
        self.save_op("LD H E");
    }
    fn ld_h_h(&mut self) {
        let op = self.get_h();
        self.set_h(op);
        self.save_op("LD H H");
    }
    fn ld_h_l(&mut self) {
        let op = self.get_l();
        self.set_h(op);
        self.save_op("LD H L");
    }
    fn ld_h_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let byte = mem.peek(addr);
        self.h = byte;
        self.save_op("LD H HL");
    }
    fn ld_h_a(&mut self) {
        let op = self.a;
        self.set_h(op);
        self.save_op("LD H A");
    }
    fn ld_l_b(&mut self) {
        let op = self.b;
        self.set_l(op);
        self.save_op("LD L B");
    }
    fn ld_l_c(&mut self) {
        let op = self.c;
        self.set_l(op);
        self.save_op("LD L C");
    }
    fn ld_l_d(&mut self) {
        let op = self.d;
        self.set_l(op);
        self.save_op("LD L D");
    }
    fn ld_l_e(&mut self) {
        let op = self.e;
        self.set_l(op);
        self.save_op("LD L E");
    }
    fn ld_l_h(&mut self) {
        let op = self.get_h();
        self.set_l(op);
        self.save_op("LD L H");
    }
    fn ld_l_l(&mut self) {
        let op = self.get_l();
        self.set_l(op);
        self.save_op("LD L L");
    }
    fn ld_l_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let byte = mem.peek(addr);
        self.l = byte;
        self.save_op("LD L HL");
    }
    fn ld_l_a(&mut self) {
        let op = self.e;
        self.set_l(op);
        self.save_op("LD L A");
    }
    fn ld_a_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.a = x1;
        let msg = format!("LD A {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_b_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.b = x1;
        let msg = format!("LD B {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_c_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.c = x1;
        let msg = format!("LD C {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_d_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.d = x1;
        let msg = format!("LD D {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_e_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.e = x1;
        let msg = format!("LD E {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_h_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.set_h(x1);
        let msg = format!("LD H {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_at_nn_a(&mut self, mem: &mut Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        let addr = Z80::get_word(x2, x1);
        mem.poke(addr, self.a);
        let msg = format!("LD ({:x}) A", x1);
        self.save_op(&msg);
    }
    fn jr_z_e(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem) as i8;
        let x2 = (self.pc as i16).wrapping_add(x1 as i16);
        if Z80::check_flag(self.f, Z) {
            self.pc = x2 as u16;
        }
        let msg = format!("JR Z {:x}", x1);
        self.save_op(&msg);
    }
    fn djnz_e(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem) as i16;
        let x2 = self.pc as i16 + x1;
        self.b -= 1;
        if self.b == 0 {
            self.pc = x2 as u16;
        }
        let msg = format!("DJNZ {:x}", x1);
        self.save_op(&msg);
    }
    fn ld_hl_nn(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        let addr = Z80::get_word(x2, x1);
        let lo = mem.peek(addr);
        let hi = mem.peek(addr + 1);
        self.set_h(hi);
        self.set_l(lo);
        let msg = format!("LD HL {:x}{:x}", x2, x1);
        self.save_op(&msg);
    }
    fn cp_r(&mut self, other: u8) {
        let diff = (self.a as i16) - (other as i16);
        let mut cond = (diff & 0x80) > 0;
        self.set_reset_flag(cond, S);
        cond = diff == 0;
        self.set_reset_flag(cond, Z);
        cond = (self.a & 0xF) < (other & 0xF);
        self.set_reset_flag(cond, H);
        cond = ((self.a > 0x80) & (other > 0x80) & (diff > 0))
            | ((self.a < 0x80) & (other < 0x80) & (diff < 0));
        self.set_reset_flag(cond, P_V);
        self.set_flag(N);

        cond = (other & BIT_3) > 0;
        self.set_reset_flag(cond, BIT_3);
        cond = (other & BIT_5) > 0;
        self.set_reset_flag(cond, BIT_5);

        cond = diff & 0x100 != 0;
        self.set_reset_flag(cond, C);
    }
    fn cp_b(&mut self) {
        let op = self.b;
        self.cp_r(op);
        self.save_op("CP B");
    }
    fn cp_c(&mut self) {
        let op = self.c;
        self.cp_r(op);
        self.save_op("CP C");
    }
    fn cp_d(&mut self) {
        let op = self.d;
        self.cp_r(op);
        self.save_op("CP D");
    }
    fn cp_e(&mut self) {
        let op = self.e;
        self.cp_r(op);
        self.save_op("CP E");
    }
    fn cp_h(&mut self) {
        let op = self.get_h();
        self.cp_r(op);
        self.save_op("CP H");
    }
    fn cp_l(&mut self) {
        let op = self.get_l();
        self.cp_r(op);
        self.save_op("CP L");
    }
    fn cp_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let op = mem.peek(addr);
        self.cp_r(op);
        self.save_op("CP (HL)");
    }
    fn cp_a(&mut self) {
        let op = self.a;
        self.cp_r(op);
        self.save_op("CP A");
    }
    fn cp_n(&mut self, mem: &Memory) {
        let op = self.read_bus(mem);
        self.cp_r(op);
        let msg = format!("CP {:x}", op);
        self.save_op(&msg);
    }
    fn ld_l_n(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        self.set_l(x1);
let msg = format!("LD H {:x}", x1);
        self.save_op(&msg);
    }
    fn jr_c_e(&mut self, mem: &Memory) {
        let e = self.read_bus(mem);
        if self.f & 0x01 == 1 {
            self.pc += e as u16;
        }
        let msg = format!("JR C {:x}", e);
        self.save_op(&msg);
    }
    fn jr_nc_e(&mut self, mem: &Memory) {
        let e = self.read_bus(mem);
        if self.f & 0x01 == 0 {
            self.pc += e as u16;
        }
        let msg = format!("JR NC {:x}", e);
        self.save_op(&msg);
    }
    fn out_n_a(&mut self, mem: &Memory) {
        let x1 = self.read_bus(mem);
        let dir = Z80::get_word(self.a, x1);
        let msg = format!("out {:x} A", x1);
        self.save_op(&msg);
        println!(
            "  TODO: poner el valor {} en la dirección {:x}",
            self.a, dir
        );
    }
    fn exx(&mut self) {
        let mut swap = self.b;
        self.b = self.b_alt;
        self.b_alt = swap;

        swap = self.c;
        self.c = self.c_alt;
        self.c_alt = swap;

        swap = self.d;
        self.d = self.d_alt;
        self.d_alt = swap;

        swap = self.e;
        self.e = self.e_alt;
        self.e_alt = swap;

        swap = self.h;
        self.h = self.h_alt;
        self.h_alt = swap;

        swap = self.l;
        self.l = self.l_alt;
        self.l_alt = swap;
        self.save_op("EXX");
    }
    fn scf(&mut self) {
        self.set_flag(C);
        self.reset_flag(N);
        self.reset_flag(H);
        self.save_op("SCF");
    }
    fn ccf(&mut self) {
        let old_c = self.f & C;
        self.set_reset_flag(old_c == 1, H);
        self.reset_flag(N);
        self.set_reset_flag(old_c == 0, C);
        self.save_op("CCF");
    }
    fn and_r(&mut self, other: u8) {
        self.a &= other;
        let mut cond = (self.a as i8) < 0;
        self.set_reset_flag(cond, S);
        cond = self.a == 0;
        self.set_reset_flag(cond, Z);
        self.set_flag(H);
        self.reset_flag(P_V);
        self.reset_flag(N);
        self.reset_flag(C);
    }
    fn and_b(&mut self) {
        let op = self.b;
        self.and_r(op);
        self.save_op("AND B");
    }
    fn and_c(&mut self) {
        let op = self.c;
        self.and_r(op);
        self.save_op("AND C");
    }
    fn and_d(&mut self) {
        let op = self.d;
        self.and_r(op);
        self.save_op("AND D");
    }
    fn and_e(&mut self) {
        let op = self.e;
        self.and_r(op);
        self.save_op("AND E");
    }
    fn and_h(&mut self) {
        let op = self.get_h();
        self.and_r(op);
        self.save_op("AND H");
    }
    fn and_l(&mut self) {
        let op = self.get_l();
        self.and_r(op);
        self.save_op("AND L");
    }
    fn and_at_hl(&mut self, mem: &Memory) {
        let addr = self.get_indirect_hl(mem);
        let op = mem.peek(addr);
        self.and_r(op);
        self.save_op("AND (HL)");
    }
    fn and_a(&mut self) {
        let op = self.a;
        self.and_r(op);
        self.save_op("AND A");
    }
    fn and_n(&mut self, mem: &Memory) {
        let op = self.read_bus(mem);
        self.and_r(op);
        let msg = format!("AND A {:x}", op);
        self.save_op(&msg);
    }
    fn ld_at_nn_bc(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.read_bus(mem), self.read_bus(mem));
        mem.poke(addr, self.c);
        mem.poke(addr + 1, self.b);
        self.save_op("LD (nn) BC");
    }
    fn ld_bc_at_nn(&mut self, mem: &Memory) {
        let lo = self.read_bus(mem);
        let hi = self.read_bus(mem);
        let addr = Z80::get_word(hi, lo);
        self.c = mem.peek(addr);
        self.b = mem.peek(addr + 1);
    }
    fn ld_de_at_nn(&mut self, mem: &Memory) {
        let lo = self.read_bus(mem);
        let hi = self.read_bus(mem);
        let addr = Z80::get_word(hi, lo);
        self.d = mem.peek(addr);
        self.e = mem.peek(addr + 1);
    }
    fn ld_hl_at_nn(&mut self, mem: &Memory) {
        let lo = self.read_bus(mem);
        let hi = self.read_bus(mem);
        let addr = Z80::get_word(hi, lo);
        self.h = mem.peek(addr);
        self.l = mem.peek(addr + 1);
    }
    fn ld_sp_at_nn(&mut self, mem: &Memory) {
        let lo = self.read_bus(mem);
        let hi = self.read_bus(mem);
        let addr = Z80::get_word(hi, lo);
        let sp_h = mem.peek(addr);
        let sp_l = mem.peek(addr + 1);
        self.sp = Z80::get_word(sp_h, sp_l);
    }
    fn ld_i_a(&mut self) {
        self.i = self.a;
        self.save_op("LD I A");
    }
    fn ld_r_a(&mut self) {
        self.r = self.a;
        self.save_op("LD R A");
    }
    fn ld_a_i(&mut self) {
        self.a = self.i;
        self.save_op("LD A I");
    }
    fn ld_a_r(&mut self) {
        self.a = self.r;
        self.save_op("LD A R");
    }
    fn sbc_hl_r(&mut self, hi: u8, lo: u8) {
        //let hl = ((self.h as i16) << 8) + (self.l as i16);
        //let r = ((hi as i16) << 8) + (lo as i16);
        let hl = Z80::get_word(self.get_h(), self.get_l()) as i16;
        let r = Z80::get_word(hi, lo) as i16;
        let c = (self.f & 0x01) as i16;

        let diff = (hl as i32) - (r as i32) - (c as i32);
        self.set_l((diff & 0xff) as u8);
        self.set_h(((diff >> 8) & 0xff) as u8);
        self.set_reset_flag(diff < 0, S);
        self.set_reset_flag(diff == 0, Z);
        let cond = self.l < (lo + (c as u8));
        self.set_reset_flag(cond, H);
        self.set_reset_flag((diff < 0) | (diff > 255), P_V);
        self.set_flag(N);
        self.set_reset_flag((diff as u16) > (hl as u16), C);
    }
    fn sbc_hl_de(&mut self) {
        let hi = self.d;
        let lo = self.e;
        self.sbc_hl_r(hi, lo);
        self.save_op("SBC HL DE")
    }
    fn sbc_hl_bc(&mut self) {
        let hi = self.b;
        let lo = self.c;
        self.sbc_hl_r(hi, lo);
        self.save_op("SBC HL BC")
    }
    fn sbc_hl_hl(&mut self) {
        let hi = self.h;
        let lo = self.l;
        self.sbc_hl_r(hi, lo);
        self.save_op("SBC HL HL")
    }
    fn sbc_hl_sp(&mut self) {
        let bytes = Z80::get_bytes(self.sp);
        self.sbc_hl_r(bytes.0, bytes.1);
        self.save_op("SBC HL SP")
    }
    fn adc_hl_r(&mut self, hi: u8, lo: u8) {
        let hl = Z80::get_word(self.h, self.l) as i16;
        let r = Z80::get_word(hi, lo) as i16;
        let c = self.get_flag(C) as i16;

        let diff = hl + r + c;
        self.l = (diff & 0xff) as u8;
        self.h = ((diff >> 8) & 0xff) as u8;
        self.set_reset_flag(diff < 0, S);
        self.set_reset_flag(diff == 0, Z);
        let cond = self.l < (lo + (c as u8));
        self.set_reset_flag(cond, H);
        self.set_reset_flag((diff < 0) | (diff > 255), P_V);
        self.set_flag(N);
        self.set_reset_flag((diff as u16) > (hl as u16), C);
    }
    fn adc_hl_de(&mut self) {
        let hi = self.d;
        let lo = self.e;
        self.adc_hl_r(hi, lo);
        self.save_op("SBC HL DE")
    }
    fn adc_hl_bc(&mut self) {
        let hi = self.b;
        let lo = self.c;
        self.adc_hl_r(hi, lo);
        self.save_op("SBC HL BC")
    }
    fn adc_hl_hl(&mut self) {
        let hi = self.h;
        let lo = self.l;
        self.adc_hl_r(hi, lo);
        self.save_op("SBC HL HL")
    }
    fn adc_hl_sp(&mut self) {
        let bytes = Z80::get_bytes(self.sp);
        self.adc_hl_r(bytes.0, bytes.1);
        self.save_op("SBC HL SP")
    }
    fn ld_nn_de(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.read_bus(mem), self.read_bus(mem));
        mem.poke(addr, self.e);
        mem.poke(addr + 1, self.d);
        self.save_op("LD (nn) DE");
    }
    fn ld_nn_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.read_bus(mem), self.read_bus(mem));
        mem.poke(addr, self.get_l());
        mem.poke(addr + 1, self.get_h());

        self.save_op("LD (nn) HL");
    }
    fn ld_nn_sp(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.read_bus(mem), self.read_bus(mem));
        mem.poke(addr, (self.sp & 0xff) as u8);
        mem.poke(addr + 1, ((self.sp >> 8) & 0xff) as u8);
        self.save_op("LD (nn) HL");
    }
    fn ret_cc(&mut self, mem: &Memory, cond: bool) {
        if cond {
            self.ret(mem);
        }
        self.save_op("RET CC");
    }
    fn ret_nz(&mut self, mem: &Memory) {
        let cond = self.f & Z == 0;
        self.ret_cc(mem, cond);
        self.save_op("RET NZ");
    }
    fn ret_z(&mut self, mem: &Memory) {
        let cond = self.f & Z != 0;
        self.ret_cc(mem, cond);
        self.save_op("RET Z");
    }
    fn ret_nc(&mut self, mem: &Memory) {
        let cond = self.f & C == 0;
        self.ret_cc(mem, cond);
        self.save_op("RET NC");
    }
    fn ret_c(&mut self, mem: &Memory) {
        let cond = self.f & C != 0;
        self.ret_cc(mem, cond);
        self.save_op("RET C");
    }
    fn ret_po(&mut self, mem: &Memory) {
        let cond = self.f & P_V == 0;
        self.ret_cc(mem, cond);
        self.save_op("RET PO");
    }
    fn ret_pe(&mut self, mem: &Memory) {
        let cond = self.f & P_V != 0;
        self.ret_cc(mem, cond);
        self.save_op("RET PE");
    }
    fn ret_p(&mut self, mem: &Memory) {
        let cond = self.f & S != 0;
        self.ret_cc(mem, cond);
        self.save_op("RET P");
    }
    fn ret_m(&mut self, mem: &Memory) {
        let cond = self.f & S == 0;
        self.ret_cc(mem, cond);
        self.save_op("RET M");
    }
    fn pop_bc(&mut self, mem: &Memory) {
        let lo = mem.peek(self.sp);
        let hi = mem.peek(self.sp + 1);
        self.sp += 2;
        self.b = hi;
        self.c = lo;
        self.save_op("POP BC");
    }
    fn pop_de(&mut self, mem: &Memory) {
        let lo = mem.peek(self.sp);
        let hi = mem.peek(self.sp + 1);
        self.sp += 2;
        self.d = hi;
        self.e = lo;
        self.save_op("POP DE");
    }
    fn pop_hl(&mut self, mem: &Memory) {
        let lo = mem.peek(self.sp);
        let hi = mem.peek(self.sp + 1);
        self.sp += 2;
        self.set_h(hi);
        self.set_l(lo);
        self.save_op("POP HL");
    }
    fn pop_af(&mut self, mem: &Memory) {
        let lo = mem.peek(self.sp);
        let hi = mem.peek(self.sp + 1);
        self.sp += 2;
        self.a = hi;
        self.f = lo;
        self.save_op("POP AF");
    }
    fn jp_cc(&mut self, cond: bool, mem: &Memory) {
        let lo = self.read_bus(mem);
        let hi = self.read_bus(mem);
        if cond {
            self.pc = Z80::get_word(hi, lo);
        }
    }
    fn jp_nz(&mut self, mem: &Memory) {
        let cond = self.f & Z == 0;
        self.jp_cc(cond, mem);
        self.save_op("JP NZ");
    }
    fn jp_z(&mut self, mem: &Memory) {
        let cond = self.f & Z != 0;
        self.jp_cc(cond, mem);
        self.save_op("JP Z");
    }
    fn jp_nc(&mut self, mem: &Memory) {
        let cond = self.f & C == 0;
        self.jp_cc(cond, mem);
        self.save_op("JP NC");
    }
    fn jp_c(&mut self, mem: &Memory) {
        let cond = self.f & C != 0;
        self.jp_cc(cond, mem);
        self.save_op("JP C");
    }
    fn jp_po(&mut self, mem: &Memory) {
        let cond = self.f & P_V == 0;
        self.jp_cc(cond, mem);
        self.save_op("JP PO");
    }
    fn jp_pe(&mut self, mem: &Memory) {
        let cond = self.f & P_V != 0;
        self.jp_cc(cond, mem);
        self.save_op("JP PE");
    }
    fn jp_m(&mut self, mem: &Memory) {
        let cond = self.f & S == 0;
        self.jp_cc(cond, mem);
        self.save_op("JP M");
    }
    fn jp_p(&mut self, mem: &Memory) {
        let cond = self.f & S != 0;
        self.jp_cc(cond, mem);
        self.save_op("JP P");
    }
    fn jp_at_hl(&mut self) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        self.sp = addr;
        self.save_op("JP (HL)");
    }
    fn call_cc_nn(&mut self, cond: bool, mem: &mut Memory) {
        let lo = self.read_bus(mem);
        let hi = self.read_bus(mem);
        if cond {
            let pc_lo = (self.pc & 0xff) as u8;
            let pc_hi = ((self.pc >> 8) & 0xff) as u8;
            mem.poke(self.sp - 1, pc_lo);
            mem.poke(self.sp - 2, pc_hi);
            self.sp -= 2;
            self.pc = Z80::get_word(hi, lo);
        }
    }
    fn call_nz(&mut self, mem: &mut Memory) {
        let cond = self.f & Z == 0;
        self.call_cc_nn(cond, mem);
    }
    fn call_z(&mut self, mem: &mut Memory) {
        let cond = self.f & Z != 0;
        self.call_cc_nn(cond, mem);
        self.save_op("JP Z");
    }
    fn call_nc(&mut self, mem: &mut Memory) {
        let cond = self.f & C == 0;
        self.call_cc_nn(cond, mem);
        self.save_op("JP NC");
    }
    fn call_c(&mut self, mem: &mut Memory) {
        let cond = self.f & C != 0;
        self.call_cc_nn(cond, mem);
        self.save_op("JP C");
    }
    fn call_po(&mut self, mem: &mut Memory) {
        let cond = self.f & P_V == 0;
        self.call_cc_nn(cond, mem);
        self.save_op("JP PO");
    }
    fn call_pe(&mut self, mem: &mut Memory) {
        let cond = self.f & P_V != 0;
        self.call_cc_nn(cond, mem);
        self.save_op("JP PE");
    }
    fn call_m(&mut self, mem: &mut Memory) {
        let cond = self.f & S == 0;
        self.call_cc_nn(cond, mem);
        self.save_op("JP M");
    }
    fn call_p(&mut self, mem: &mut Memory) {
        let cond = self.f & S != 0;
        self.call_cc_nn(cond, mem);
        self.save_op("JP P");
    }
    fn push_qq(&mut self, mem: &mut Memory, hi: u8, lo: u8) {
        mem.poke(self.sp - 1, lo);
        mem.poke(self.sp - 2, hi);
        self.sp -= 2;
    }
    fn push_bc(&mut self, mem: &mut Memory) {
        let hi = self.b;
        let lo = self.c;
        self.push_qq(mem, hi, lo);
        self.save_op("PUSH BC");
    }
    fn push_de(&mut self, mem: &mut Memory) {
        let hi = self.d;
        let lo = self.e;
        self.push_qq(mem, hi, lo);
        self.save_op("PUSH DE");
    }
    fn push_hl(&mut self, mem: &mut Memory) {
        let hi = self.get_h();
        let lo = self.get_l();
        self.push_qq(mem, hi, lo);
        self.save_op("PUSH HL");
    }
    fn push_af(&mut self, mem: &mut Memory) {
        let hi = self.a;
        let lo = self.f;
        self.push_qq(mem, hi, lo);
        self.save_op("PUSH AF");
    }
    fn add_a_n(&mut self, mem: &Memory) {
        let n: u8 = self.read_bus(mem);
        let sum: i16 = (self.a as i16) + (n as i16);
        self.a = (sum & 0xff) as u8;
        self.set_reset_flag(((sum & 0xff) as i8) < 0, S);
        self.set_reset_flag(sum == 0, Z);
        self.set_reset_flag((sum - 1) & 0xf == 0xf, H);
        self.set_reset_flag(sum > (sum & 0xff), P_V);
        self.reset_flag(N);
        self.set_reset_flag((sum - 1) & 0xff == 0xff, C);
    }
    fn rst_n(&mut self, mem: &mut Memory, new_pc: u16) {
        self.sp -= 1;
        let mut val = (self.pc >> 8) as u8;
        let mut addr = self.sp;
        mem.poke(addr, val);
        val = (self.pc & 0xff) as u8;
        addr -= 1;
        mem.poke(addr, val);
        self.pc = new_pc;
    }
    fn rst_0(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 0);
        self.save_op("RST 00");
    }
    fn rst_8(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 8);
        self.save_op("RST 08");
    }
    fn rst_10(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 10);
        self.save_op("RST 10");
    }
    fn rst_18(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 18);
        self.save_op("RST 18");
    }
    fn rst_20(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 20);
        self.save_op("RST 20");
    }
    fn rst_28(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 28);
        self.save_op("RST 28");
    }
    fn rst_30(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 30);
        self.save_op("RST 30");
    }
    fn rst_38(&mut self, mem: &mut Memory) {
        self.rst_n(mem, 38);
        self.save_op("RST 38");
    }
    fn ret(&mut self, mem: &Memory) {
        let lo = mem.peek(self.sp);
        let hi = mem.peek(self.sp + 1);
        let new_pc = Z80::get_word(hi, lo);
        self.pc = new_pc;
        self.sp += 2;
        self.save_op("RET");
    }
    fn call_nn(&mut self, mem: &mut Memory) {
        let lo = mem.peek(self.sp);
        let hi = mem.peek(self.sp + 1);
        let mut val = (self.pc >> 8) as u8;
        let mut addr = self.sp - 1;
        mem.poke(addr, val);
        val = (self.pc & 0xff) as u8;
        addr = self.sp - 2;
        mem.poke(addr, val);
        self.sp -= 2;

        let new_pc = Z80::get_word(hi, lo);
        self.pc = new_pc;
        self.save_op("CALL nn");
    }
    fn ex_at_sp_hl(&mut self, mem: &mut Memory) {
        let new_l = mem.peek(self.sp);
        let old_l = self.get_l();
        self.set_l(new_l);
        mem.poke(self.sp, old_l);
        let new_h = mem.peek(self.sp + 1);
        let old_h = self.get_h();
        self.set_h(new_h);
        mem.poke(self.sp, old_h);

        self.save_op("EX (SP) HL");
    }
    fn ei(&mut self) {
        self.iff1 = true;
        self.iff2 = true;
        self.save_op("EI");
    }
    fn rlc_r(&mut self, r: u8) -> u8 {
        let new_bit_0 = (r & 0x80) >> 7;
        let result = (r << 1) + new_bit_0;
        self.set_reset_flag(new_bit_0 > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn rlc_b(&mut self) {
        let op = self.b;
        self.b = self.rlc_r(op);
        self.save_op("RLC B");
    }
    fn rlc_c(&mut self) {
        let op = self.c;
        self.c = self.rlc_r(op);
        self.save_op("RLC C");
    }
    fn rlc_d(&mut self) {
        let op = self.d;
        self.d = self.rlc_r(op);
        self.save_op("RLC D");
    }
    fn rlc_e(&mut self) {
        let op = self.e;
        self.e = self.rlc_r(op);
        self.save_op("RLC E");
    }
    fn rlc_h(&mut self) {
        let op = self.h;
        self.h = self.rlc_r(op);
        self.save_op("RLC H");
    }
    fn rlc_l(&mut self) {
        let op = self.l;
        self.l = self.rlc_r(op);
        self.save_op("RLC L");
    }
    fn rlc_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.rlc_r(op);
        mem.poke(addr, new_data);
        self.save_op("RLC L");
    }
    fn rlc_a(&mut self) {
        let op = self.a;
        self.a = self.rlc_r(op);
        self.save_op("RLC L");
    }
    fn rrc_r(&mut self, r: u8) -> u8 {
        let new_bit_8 = r & 0x01;
        let result = (r >> 1) + (new_bit_8 << 7);
        self.set_reset_flag(new_bit_8 > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn rrc_b(&mut self) {
        let op = self.b;
        self.b = self.rrc_r(op);
        self.save_op("RLC B");
    }
    fn rrc_c(&mut self) {
        let op = self.c;
        self.c = self.rrc_r(op);
        self.save_op("RLC C");
    }
    fn rrc_d(&mut self) {
        let op = self.d;
        self.d = self.rrc_r(op);
        self.save_op("RLC D");
    }
    fn rrc_e(&mut self) {
        let op = self.e;
        self.e = self.rrc_r(op);
        self.save_op("RLC E");
    }
    fn rrc_h(&mut self) {
        let op = self.h;
        self.h = self.rrc_r(op);
        self.save_op("RLC H");
    }
    fn rrc_l(&mut self) {
        let op = self.l;
        self.l = self.rrc_r(op);
        self.save_op("RLC L");
    }
    fn rrc_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.rrc_r(op);
        mem.poke(addr, new_data);
        self.save_op("RLC L");
    }
    fn rrc_a(&mut self) {
        let op = self.a;
        self.a = self.rrc_r(op);
        self.save_op("RLC L");
    }
    fn rl_r(&mut self, r: u8) -> u8 {
        let new_bit_0 = self.get_flag(C);
        let new_carry = (r & 0x80) >> 7;
        let result = (r << 1) + new_bit_0;
        self.set_reset_flag(new_carry > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn rl_b(&mut self) {
        let op = self.b;
        self.b = self.rl_r(op);
        self.save_op("RL B");
    }
    fn rl_c(&mut self) {
        let op = self.c;
        self.c = self.rl_r(op);
        self.save_op("RL C");
    }
    fn rl_d(&mut self) {
        let op = self.d;
        self.d = self.rl_r(op);
        self.save_op("RL D");
    }
    fn rl_e(&mut self) {
        let op = self.e;
        self.e = self.rl_r(op);
        self.save_op("RL E");
    }
    fn rl_h(&mut self) {
        let op = self.h;
        self.h = self.rl_r(op);
        self.save_op("RL H");
    }
    fn rl_l(&mut self) {
        let op = self.l;
        self.l = self.rl_r(op);
        self.save_op("RL L");
    }
    fn rl_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.rl_r(op);
        mem.poke(addr, new_data);
        self.save_op("RL L");
    }
    fn rl_a(&mut self) {
        let op = self.a;
        self.a = self.rl_r(op);
        self.save_op("RL L");
    }
    fn rr_r(&mut self, r: u8) -> u8 {
        let new_bit_8 = self.get_flag(C);
        let new_carry = r & 0x01;
        let result = (r >> 1) + (new_bit_8 << 7);
        self.set_reset_flag(new_carry > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn rr_b(&mut self) {
        let op = self.b;
        self.b = self.rr_r(op);
        self.save_op("RR B");
    }
    fn rr_c(&mut self) {
        let op = self.c;
        self.c = self.rr_r(op);
        self.save_op("RR C");
    }
    fn rr_d(&mut self) {
        let op = self.d;
        self.d = self.rr_r(op);
        self.save_op("RR D");
    }
    fn rr_e(&mut self) {
        let op = self.e;
        self.e = self.rr_r(op);
        self.save_op("RR E");
    }
    fn rr_h(&mut self) {
        let op = self.h;
        self.h = self.rr_r(op);
        self.save_op("RR H");
    }
    fn rr_l(&mut self) {
        let op = self.l;
        self.l = self.rr_r(op);
        self.save_op("RR L");
    }
    fn rr_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.rr_r(op);
        mem.poke(addr, new_data);
        self.save_op("RR L");
    }
    fn rr_a(&mut self) {
        let op = self.a;
        self.a = self.rr_r(op);
        self.save_op("RR L");
    }
    fn sla_r(&mut self, r: u8) -> u8 {
        let new_carry = (r & 0x80) >> 7;
        let result = r << 1;
        self.set_reset_flag(new_carry > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn sla_b(&mut self) {
        let op = self.b;
        self.b = self.sla_r(op);
        self.save_op("SLA B");
    }
    fn sla_c(&mut self) {
        let op = self.c;
        self.c = self.sla_r(op);
        self.save_op("SLA C");
    }
    fn sla_d(&mut self) {
        let op = self.d;
        self.d = self.sla_r(op);
        self.save_op("SLA D");
    }
    fn sla_e(&mut self) {
        let op = self.e;
        self.e = self.sla_r(op);
        self.save_op("SLA E");
    }
    fn sla_h(&mut self) {
        let op = self.h;
        self.h = self.sla_r(op);
        self.save_op("SLA H");
    }
    fn sla_l(&mut self) {
        let op = self.l;
        self.l = self.sla_r(op);
        self.save_op("SLA L");
    }
    fn sla_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.sla_r(op);
        mem.poke(addr, new_data);
        self.save_op("SLA L");
    }
    fn sla_a(&mut self) {
        let op = self.a;
        self.a = self.sla_r(op);
        self.save_op("SLA L");
    }
    fn sra_r(&mut self, r: u8) -> u8 {
        let new_carry = r & 0x01;
        let new_bit_8 = r & 0x80;
        let result = r >> 1 + new_bit_8;
        self.set_reset_flag(new_carry > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn sra_b(&mut self) {
        let op = self.b;
        self.b = self.sra_r(op);
        self.save_op("SRA B");
    }
    fn sra_c(&mut self) {
        let op = self.c;
        self.c = self.sra_r(op);
        self.save_op("SRA C");
    }
    fn sra_d(&mut self) {
        let op = self.d;
        self.d = self.sra_r(op);
        self.save_op("SRA D");
    }
    fn sra_e(&mut self) {
        let op = self.e;
        self.e = self.sra_r(op);
        self.save_op("SRA E");
    }
    fn sra_h(&mut self) {
        let op = self.h;
        self.h = self.sra_r(op);
        self.save_op("SRA H");
    }
    fn sra_l(&mut self) {
        let op = self.l;
        self.l = self.sra_r(op);
        self.save_op("SRA L");
    }
    fn sra_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.sra_r(op);
        mem.poke(addr, new_data);
        self.save_op("SRA L");
    }
    fn sra_a(&mut self) {
        let op = self.a;
        self.a = self.sra_r(op);
        self.save_op("SRA L");
    }
    fn sll_r(&mut self, r: u8) -> u8 {
        let new_carry = (r & 0x80) >> 7;
        let result = r >> 1 + 1;
        self.set_reset_flag(new_carry > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn sll_b(&mut self) {
        let op = self.b;
        self.b = self.sll_r(op);
        self.save_op("SLL B");
    }
    fn sll_c(&mut self) {
        let op = self.c;
        self.c = self.sll_r(op);
        self.save_op("SLL C");
    }
    fn sll_d(&mut self) {
        let op = self.d;
        self.d = self.sll_r(op);
        self.save_op("SLL D");
    }
    fn sll_e(&mut self) {
        let op = self.e;
        self.e = self.sll_r(op);
        self.save_op("SLL E");
    }
    fn sll_h(&mut self) {
        let op = self.h;
        self.h = self.sll_r(op);
        self.save_op("SLL H");
    }
    fn sll_l(&mut self) {
        let op = self.l;
        self.l = self.sll_r(op);
        self.save_op("SLL L");
    }
    fn sll_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.sll_r(op);
        mem.poke(addr, new_data);
        self.save_op("SLL L");
    }
    fn sll_a(&mut self) {
        let op = self.a;
        self.a = self.sll_r(op);
        self.save_op("SLL A");
    }
    fn srl_r(&mut self, r: u8) -> u8 {
        let new_carry = r & 0x01;
        let result = r >> 1;
        self.set_reset_flag(new_carry > 0, C);
        self.reset_flag(H);
        self.reset_flag(N);
        self.set_reset_flag((result as i8) < 0, S);
        self.set_reset_flag(result == 0, Z);
        self.set_reset_flag(Z80::check_byte_parity(result), P_V);
        result
    }
    fn srl_b(&mut self) {
        let op = self.b;
        self.b = self.srl_r(op);
        self.save_op("SRL B");
    }
    fn srl_c(&mut self) {
        let op = self.c;
        self.c = self.srl_r(op);
        self.save_op("SRL C");
    }
    fn srl_d(&mut self) {
        let op = self.d;
        self.d = self.srl_r(op);
        self.save_op("SRL D");
    }
    fn srl_e(&mut self) {
        let op = self.e;
        self.e = self.srl_r(op);
        self.save_op("SRL E");
    }
    fn srl_h(&mut self) {
        let op = self.h;
        self.h = self.srl_r(op);
        self.save_op("SRL H");
    }
    fn srl_l(&mut self) {
        let op = self.l;
        self.l = self.srl_r(op);
        self.save_op("SRL L");
    }
    fn srl_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.get_h(), self.get_l());
        let op = mem.peek(addr);
        let new_data = self.srl_r(op);
        mem.poke(addr, new_data);
        self.save_op("SRL (HL)");
    }
    fn srl_a(&mut self) {
        let op = self.a;
        self.a = self.srl_r(op);
        self.save_op("SRL A");
    }
    fn res_n_r(bit_num: u8, val: u8) -> u8 {
        let mask = !(0x1 << bit_num);
        val & mask
    }
    fn set_n_r(bit_num: u8, val: u8) -> u8 {
        let mask = 0x1 << bit_num;
        val | mask
    }
    fn bit_n_r(&mut self, n: u8, r: u8) {
        let mask = 0x01 << n;
        let comparison = r & mask;
        let new_z = comparison >> n;
        self.set_reset_flag(new_z > 0, Z);
        self.set_flag(H);
        self.reset_flag(N);
    }
    fn bit_0_b(&mut self) {
        let op = self.b;
        self.bit_n_r(0, op);
        self.save_op("BIT 0 B")
    }
    fn bit_0_c(&mut self) {
        let op = self.c;
        self.bit_n_r(0, op);
        self.save_op("BIT 0 C")
    }
    fn bit_0_d(&mut self) {
        let op = self.d;
        self.bit_n_r(0, op);
        self.save_op("BIT 0 D")
    }
    fn bit_0_e(&mut self) {
        let op = self.e;
        self.bit_n_r(0, op);
        self.save_op("BIT 0 E")
    }
    fn bit_0_h(&mut self) {
        let op = self.h;
        self.bit_n_r(0, op);
        self.save_op("BIT 0 H")
    }
    fn bit_0_l(&mut self) {
        let op = self.l;
        self.bit_n_r(0, op);
        self.save_op("BIT 0 L")
    }
    fn bit_0_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(0, op);
        self.save_op("BIT 0 (HL)")
    }
    fn bit_0_a(&mut self) {
        let op = self.a;
        self.bit_n_r(0, op);
        self.save_op("BIT 0 AL")
    }
    fn bit_1_b(&mut self) {
        let op = self.b;
        self.bit_n_r(1, op);
        self.save_op("BIT 1 B")
    }
    fn bit_1_c(&mut self) {
        let op = self.c;
        self.bit_n_r(1, op);
        self.save_op("BIT 1 C")
    }
    fn bit_1_d(&mut self) {
        let op = self.d;
        self.bit_n_r(1, op);
        self.save_op("BIT 1 D")
    }
    fn bit_1_e(&mut self) {
        let op = self.e;
        self.bit_n_r(1, op);
        self.save_op("BIT 1 E")
    }
    fn bit_1_h(&mut self) {
        let op = self.h;
        self.bit_n_r(1, op);
        self.save_op("BIT 1 H")
    }
    fn bit_1_l(&mut self) {
        let op = self.l;
        self.bit_n_r(1, op);
        self.save_op("BIT 1 L")
    }
    fn bit_1_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(1, op);
        self.save_op("BIT 1 (HL)")
    }
    fn bit_1_a(&mut self) {
        let op = self.a;
        self.bit_n_r(1, op);
        self.save_op("BIT 1 AL")
    }
    fn bit_2_b(&mut self) {
        let op = self.b;
        self.bit_n_r(2, op);
        self.save_op("BIT 2 B")
    }
    fn bit_2_c(&mut self) {
        let op = self.c;
        self.bit_n_r(2, op);
        self.save_op("BIT 2 C")
    }
    fn bit_2_d(&mut self) {
        let op = self.d;
        self.bit_n_r(2, op);
        self.save_op("BIT 2 D")
    }
    fn bit_2_e(&mut self) {
        let op = self.e;
        self.bit_n_r(2, op);
        self.save_op("BIT 2 E")
    }
    fn bit_2_h(&mut self) {
        let op = self.h;
        self.bit_n_r(2, op);
        self.save_op("BIT 2 H")
    }
    fn bit_2_l(&mut self) {
        let op = self.l;
        self.bit_n_r(2, op);
        self.save_op("BIT 2 L")
    }
    fn bit_2_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(2, op);
        self.save_op("BIT 2 (HL)")
    }
    fn bit_2_a(&mut self) {
        let op = self.a;
        self.bit_n_r(2, op);
        self.save_op("BIT 2 AL")
    }
    fn bit_3_b(&mut self) {
        let op = self.b;
        self.bit_n_r(3, op);
        self.save_op("BIT 3 B")
    }
    fn bit_3_c(&mut self) {
        let op = self.c;
        self.bit_n_r(3, op);
        self.save_op("BIT 3 C")
    }
    fn bit_3_d(&mut self) {
        let op = self.d;
        self.bit_n_r(3, op);
        self.save_op("BIT 3 D")
    }
    fn bit_3_e(&mut self) {
        let op = self.e;
        self.bit_n_r(3, op);
        self.save_op("BIT 3 E")
    }
    fn bit_3_h(&mut self) {
        let op = self.h;
        self.bit_n_r(3, op);
        self.save_op("BIT 3 H")
    }
    fn bit_3_l(&mut self) {
        let op = self.l;
        self.bit_n_r(3, op);
        self.save_op("BIT 3 L")
    }
    fn bit_3_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(3, op);
        self.save_op("BIT 3 (HL)")
    }
    fn bit_3_a(&mut self) {
        let op = self.a;
        self.bit_n_r(3, op);
        self.save_op("BIT 3 AL")
    }
    fn bit_4_b(&mut self) {
        let op = self.b;
        self.bit_n_r(4, op);
        self.save_op("BIT 4 B")
    }
    fn bit_4_c(&mut self) {
        let op = self.c;
        self.bit_n_r(4, op);
        self.save_op("BIT 4 C")
    }
    fn bit_4_d(&mut self) {
        let op = self.d;
        self.bit_n_r(4, op);
        self.save_op("BIT 4 D")
    }
    fn bit_4_e(&mut self) {
        let op = self.e;
        self.bit_n_r(4, op);
        self.save_op("BIT 4 E")
    }
    fn bit_4_h(&mut self) {
        let op = self.h;
        self.bit_n_r(4, op);
        self.save_op("BIT 4 H")
    }
    fn bit_4_l(&mut self) {
        let op = self.l;
        self.bit_n_r(4, op);
        self.save_op("BIT 4 L")
    }
    fn bit_4_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(4, op);
        self.save_op("BIT 4 (HL)")
    }
    fn bit_4_a(&mut self) {
        let op = self.a;
        self.bit_n_r(4, op);
        self.save_op("BIT 4 AL")
    }
    fn bit_5_b(&mut self) {
        let op = self.b;
        self.bit_n_r(5, op);
        self.save_op("BIT 5 B")
    }
    fn bit_5_c(&mut self) {
        let op = self.c;
        self.bit_n_r(5, op);
        self.save_op("BIT 5 C")
    }
    fn bit_5_d(&mut self) {
        let op = self.d;
        self.bit_n_r(5, op);
        self.save_op("BIT 5 D")
    }
    fn bit_5_e(&mut self) {
        let op = self.e;
        self.bit_n_r(5, op);
        self.save_op("BIT 5 E")
    }
    fn bit_5_h(&mut self) {
        let op = self.h;
        self.bit_n_r(5, op);
        self.save_op("BIT 5 H")
    }
    fn bit_5_l(&mut self) {
        let op = self.l;
        self.bit_n_r(5, op);
        self.save_op("BIT 5 L")
    }
    fn bit_5_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(5, op);
        self.save_op("BIT 5 (HL)")
    }
    fn bit_5_a(&mut self) {
        let op = self.a;
        self.bit_n_r(5, op);
        self.save_op("BIT 5 AL")
    }
    fn bit_6_b(&mut self) {
        let op = self.b;
        self.bit_n_r(6, op);
        self.save_op("BIT 6 B")
    }
    fn bit_6_c(&mut self) {
        let op = self.c;
        self.bit_n_r(6, op);
        self.save_op("BIT 6 C")
    }
    fn bit_6_d(&mut self) {
        let op = self.d;
        self.bit_n_r(6, op);
        self.save_op("BIT 6 D")
    }
    fn bit_6_e(&mut self) {
        let op = self.e;
        self.bit_n_r(6, op);
        self.save_op("BIT 6 E")
    }
    fn bit_6_h(&mut self) {
        let op = self.h;
        self.bit_n_r(6, op);
        self.save_op("BIT 6 H")
    }
    fn bit_6_l(&mut self) {
        let op = self.l;
        self.bit_n_r(6, op);
        self.save_op("BIT 6 L")
    }
    fn bit_6_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(6, op);
        self.save_op("BIT 6 (HL)")
    }
    fn bit_6_a(&mut self) {
        let op = self.a;
        self.bit_n_r(6, op);
        self.save_op("BIT 6 AL")
    }
    fn bit_7_b(&mut self) {
        let op = self.b;
        self.bit_n_r(7, op);
        self.save_op("BIT 7 B")
    }
    fn bit_7_c(&mut self) {
        let op = self.c;
        self.bit_n_r(7, op);
        self.save_op("BIT 7 C")
    }
    fn bit_7_d(&mut self) {
        let op = self.d;
        self.bit_n_r(7, op);
        self.save_op("BIT 7 D")
    }
    fn bit_7_e(&mut self) {
        let op = self.e;
        self.bit_n_r(7, op);
        self.save_op("BIT 7 E")
    }
    fn bit_7_h(&mut self) {
        let op = self.h;
        self.bit_n_r(7, op);
        self.save_op("BIT 7 H")
    }
    fn bit_7_l(&mut self) {
        let op = self.l;
        self.bit_n_r(7, op);
        self.save_op("BIT 7 L")
    }
    fn bit_7_at_hl(&mut self, mem: &Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        self.bit_n_r(7, op);
        self.save_op("BIT 7 (HL)")
    }
    fn bit_7_a(&mut self) {
        let op = self.a;
        self.bit_n_r(7, op);
        self.save_op("BIT 7 AL")
    }
    fn res_0_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(0, op);
        self.save_op("res 0 B")
    }
    fn res_0_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(0, op);
        self.save_op("res 0 C")
    }
    fn res_0_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(0, op);
        self.save_op("res 0 D")
    }
    fn res_0_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(0, op);
        self.save_op("res 0 E")
    }
    fn res_0_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(0, op);
        self.save_op("res 0 H")
    }
    fn res_0_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(0, op);
        self.save_op("res 0 L")
    }
    fn res_0_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(0, op);
        mem.poke(addr, new_val);
        self.save_op("res 0 (HL)")
    }
    fn res_0_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(0, op);
        self.save_op("res 0 AL")
    }
    fn res_1_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(1, op);
        self.save_op("res 1 B")
    }
    fn res_1_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(1, op);
        self.save_op("res 1 C")
    }
    fn res_1_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(1, op);
        self.save_op("res 1 D")
    }
    fn res_1_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(1, op);
        self.save_op("res 1 E")
    }
    fn res_1_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(1, op);
        self.save_op("res 1 H")
    }
    fn res_1_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(1, op);
        self.save_op("res 1 L")
    }
    fn res_1_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(1, op);
        mem.poke(addr, new_val);
        self.save_op("res 1 (HL)")
    }
    fn res_1_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(1, op);
        self.save_op("res 1 AL")
    }
    fn res_2_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(2, op);
        self.save_op("res 2 B")
    }
    fn res_2_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(2, op);
        self.save_op("res 2 C")
    }
    fn res_2_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(2, op);
        self.save_op("res 2 D")
    }
    fn res_2_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(2, op);
        self.save_op("res 2 E")
    }
    fn res_2_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(2, op);
        self.save_op("res 2 H")
    }
    fn res_2_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(2, op);
        self.save_op("res 2 L")
    }
    fn res_2_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(2, op);
        mem.poke(addr, new_val);
        self.save_op("res 2 (HL)")
    }
    fn res_2_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(2, op);
        self.save_op("res 2 AL")
    }
    fn res_3_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(3, op);
        self.save_op("res 3 B")
    }
    fn res_3_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(3, op);
        self.save_op("res 3 C")
    }
    fn res_3_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(3, op);
        self.save_op("res 3 D")
    }
    fn res_3_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(3, op);
        self.save_op("res 3 E")
    }
    fn res_3_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(3, op);
        self.save_op("res 3 H")
    }
    fn res_3_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(3, op);
        self.save_op("res 3 L")
    }
    fn res_3_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(3, op);
        mem.poke(addr, new_val);
        self.save_op("res 3 (HL)")
    }
    fn res_3_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(3, op);
        self.save_op("res 3 AL")
    }
    fn res_4_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(4, op);
        self.save_op("res 4 B")
    }
    fn res_4_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(4, op);
        self.save_op("res 4 C")
    }
    fn res_4_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(4, op);
        self.save_op("res 4 D")
    }
    fn res_4_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(4, op);
        self.save_op("res 4 E")
    }
    fn res_4_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(4, op);
        self.save_op("res 4 H")
    }
    fn res_4_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(4, op);
        self.save_op("res 4 L")
    }
    fn res_4_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(4, op);
        mem.poke(addr, new_val);
        self.save_op("res 4 (HL)")
    }
    fn res_4_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(4, op);
        self.save_op("res 4 AL")
    }
    fn res_5_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(5, op);
        self.save_op("res 5 B")
    }
    fn res_5_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(5, op);
        self.save_op("res 5 C")
    }
    fn res_5_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(5, op);
        self.save_op("res 5 D")
    }
    fn res_5_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(5, op);
        self.save_op("res 5 E")
    }
    fn res_5_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(5, op);
        self.save_op("res 5 H")
    }
    fn res_5_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(5, op);
        self.save_op("res 5 L")
    }
    fn res_5_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(5, op);
        mem.poke(addr, new_val);
        self.save_op("res 5 (HL)")
    }
    fn res_5_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(5, op);
        self.save_op("res 5 AL")
    }
    fn res_6_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(6, op);
        self.save_op("res 6 B")
    }
    fn res_6_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(6, op);
        self.save_op("res 6 C")
    }
    fn res_6_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(6, op);
        self.save_op("res 6 D")
    }
    fn res_6_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(6, op);
        self.save_op("res 6 E")
    }
    fn res_6_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(6, op);
        self.save_op("res 6 H")
    }
    fn res_6_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(6, op);
        self.save_op("res 6 L")
    }
    fn res_6_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(6, op);
        mem.poke(addr, new_val);
        self.save_op("res 6 (HL)")
    }
    fn res_6_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(6, op);
        self.save_op("res 6 AL")
    }
    fn res_7_b(&mut self) {
        let op = self.b;
        self.b = Z80::res_n_r(7, op);
        self.save_op("res 7 B")
    }
    fn res_7_c(&mut self) {
        let op = self.c;
        self.c = Z80::res_n_r(7, op);
        self.save_op("res 7 C")
    }
    fn res_7_d(&mut self) {
        let op = self.d;
        self.d = Z80::res_n_r(7, op);
        self.save_op("res 7 D")
    }
    fn res_7_e(&mut self) {
        let op = self.e;
        self.e = Z80::res_n_r(7, op);
        self.save_op("res 7 E")
    }
    fn res_7_h(&mut self) {
        let op = self.h;
        self.h = Z80::res_n_r(7, op);
        self.save_op("res 7 H")
    }
    fn res_7_l(&mut self) {
        let op = self.l;
        self.l = Z80::res_n_r(7, op);
        self.save_op("res 7 L")
    }
    fn res_7_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::res_n_r(7, op);
        mem.poke(addr, new_val);
        self.save_op("res 7 (HL)")
    }
    fn res_7_a(&mut self) {
        let op = self.a;
        self.a = Z80::res_n_r(7, op);
        self.save_op("res 7 AL")
    }
    fn set_7_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(7, op);
        self.save_op("set 7 B")
    }
    fn set_7_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(7, op);
        self.save_op("set 7 C")
    }
    fn set_7_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(7, op);
        self.save_op("set 7 D")
    }
    fn set_7_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(7, op);
        self.save_op("set 7 E")
    }
    fn set_7_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(7, op);
        self.save_op("set 7 H")
    }
    fn set_7_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(7, op);
        self.save_op("set 7 L")
    }
    fn set_7_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(7, op);
        mem.poke(addr, new_val);
        self.save_op("set 7 (HL)")
    }
    fn set_7_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(7, op);
        self.save_op("set 7 AL")
    }
    fn set_6_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(6, op);
        self.save_op("set 6 B")
    }
    fn set_6_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(6, op);
        self.save_op("set 6 C")
    }
    fn set_6_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(6, op);
        self.save_op("set 6 D")
    }
    fn set_6_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(6, op);
        self.save_op("set 6 E")
    }
    fn set_6_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(6, op);
        self.save_op("set 6 H")
    }
    fn set_6_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(6, op);
        self.save_op("set 6 L")
    }
    fn set_6_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(6, op);
        mem.poke(addr, new_val);
        self.save_op("set 6 (HL)")
    }
    fn set_6_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(6, op);
        self.save_op("set 6 AL")
    }
    fn set_5_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(5, op);
        self.save_op("set 5 B")
    }
    fn set_5_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(5, op);
        self.save_op("set 5 C")
    }
    fn set_5_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(5, op);
        self.save_op("set 5 D")
    }
    fn set_5_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(5, op);
        self.save_op("set 5 E")
    }
    fn set_5_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(5, op);
        self.save_op("set 5 H")
    }
    fn set_5_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(5, op);
        self.save_op("set 5 L")
    }
    fn set_5_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(5, op);
        mem.poke(addr, new_val);
        self.save_op("set 5 (HL)")
    }
    fn set_5_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(5, op);
        self.save_op("set 5 AL")
    }
    fn set_4_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(4, op);
        self.save_op("set 4 B")
    }
    fn set_4_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(4, op);
        self.save_op("set 4 C")
    }
    fn set_4_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(4, op);
        self.save_op("set 4 D")
    }
    fn set_4_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(4, op);
        self.save_op("set 4 E")
    }
    fn set_4_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(4, op);
        self.save_op("set 4 H")
    }
    fn set_4_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(4, op);
        self.save_op("set 4 L")
    }
    fn set_4_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(4, op);
        mem.poke(addr, new_val);
        self.save_op("set 4 (HL)")
    }
    fn set_4_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(4, op);
        self.save_op("set 4 AL")
    }
    fn set_3_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(3, op);
        self.save_op("set 3 B")
    }
    fn set_3_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(3, op);
        self.save_op("set 3 C")
    }
    fn set_3_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(3, op);
        self.save_op("set 3 D")
    }
    fn set_3_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(3, op);
        self.save_op("set 3 E")
    }
    fn set_3_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(3, op);
        self.save_op("set 3 H")
    }
    fn set_3_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(3, op);
        self.save_op("set 3 L")
    }
    fn set_3_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(3, op);
        mem.poke(addr, new_val);
        self.save_op("set 3 (HL)")
    }
    fn set_3_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(3, op);
        self.save_op("set 3 AL")
    }
    fn set_2_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(2, op);
        self.save_op("set 2 B")
    }
    fn set_2_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(2, op);
        self.save_op("set 2 C")
    }
    fn set_2_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(2, op);
        self.save_op("set 2 D")
    }
    fn set_2_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(2, op);
        self.save_op("set 2 E")
    }
    fn set_2_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(2, op);
        self.save_op("set 2 H")
    }
    fn set_2_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(2, op);
        self.save_op("set 2 L")
    }
    fn set_2_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(2, op);
        mem.poke(addr, new_val);
        self.save_op("set 2 (HL)")
    }
    fn set_2_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(2, op);
        self.save_op("set 2 AL")
    }
    fn set_1_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(1, op);
        self.save_op("set 1 B")
    }
    fn set_1_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(1, op);
        self.save_op("set 1 C")
    }
    fn set_1_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(1, op);
        self.save_op("set 1 D")
    }
    fn set_1_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(1, op);
        self.save_op("set 1 E")
    }
    fn set_1_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(1, op);
        self.save_op("set 1 H")
    }
    fn set_1_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(1, op);
        self.save_op("set 1 L")
    }
    fn set_1_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(1, op);
        mem.poke(addr, new_val);
        self.save_op("set 1 (HL)")
    }
    fn set_1_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(1, op);
        self.save_op("set 1 AL")
    }
    fn set_0_b(&mut self) {
        let op = self.b;
        self.b = Z80::set_n_r(0, op);
        self.save_op("set 0 B")
    }
    fn set_0_c(&mut self) {
        let op = self.c;
        self.c = Z80::set_n_r(0, op);
        self.save_op("set 0 C")
    }
    fn set_0_d(&mut self) {
        let op = self.d;
        self.d = Z80::set_n_r(0, op);
        self.save_op("set 0 D")
    }
    fn set_0_e(&mut self) {
        let op = self.e;
        self.e = Z80::set_n_r(0, op);
        self.save_op("set 0 E")
    }
    fn set_0_h(&mut self) {
        let op = self.h;
        self.h = Z80::set_n_r(0, op);
        self.save_op("set 0 H")
    }
    fn set_0_l(&mut self) {
        let op = self.l;
        self.l = Z80::set_n_r(0, op);
        self.save_op("set 0 L")
    }
    fn set_0_at_hl(&mut self, mem: &mut Memory) {
        let addr = Z80::get_word(self.h, self.l);
        let op = mem.peek(addr);
        let new_val = Z80::set_n_r(0, op);
        mem.poke(addr, new_val);
        self.save_op("set 0 (HL)")
    }
    fn set_0_a(&mut self) {
        let op = self.a;
        self.a = Z80::set_n_r(0, op);
        self.save_op("set 0 AL")
    }
    fn rlc_to_b(&mut self, op: u8) {
        self.b = self.rlc_r(op);
        self.save_op("RLC (IX + d) B");
    }
    fn rlc_to_c(&mut self, op: u8) {
        self.c = self.rlc_r(op);
        self.save_op("RLC (IX + d) C");
    }
    fn rlc_to_d(&mut self, op: u8) {
        self.d = self.rlc_r(op);
        self.save_op("RLC (IX + d) D");
    }
    fn rlc_to_e(&mut self, op: u8) {
        self.e = self.rlc_r(op);
        self.save_op("RLC (IX + d) E");
    }
    fn rlc_to_h(&mut self, op: u8) {
        self.h = self.rlc_r(op);
        self.save_op("RLC (IX + d) H");
    }
    fn rlc_to_l(&mut self, op: u8) {
        self.l = self.rlc_r(op);
        self.save_op("RLC (IX + d) L");
    }
    fn rlc_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.rlc_r(op);
        mem.poke(addr, new_op);
        self.save_op("RLC (IX + d)");
    }
    fn rlc_to_a(&mut self, op: u8) {
        self.a = self.rlc_r(op);
        self.save_op("RLC (IX + d) L");
    }
    fn rrc_to_b(&mut self, op: u8) {
        self.b = self.rrc_r(op);
        self.save_op("RRC (IX + d) B");
    }
    fn rrc_to_c(&mut self, op: u8) {
        self.c = self.rrc_r(op);
        self.save_op("RRC (IX + d) C");
    }
    fn rrc_to_d(&mut self, op: u8) {
        self.d = self.rrc_r(op);
        self.save_op("RRC (IX + d) D");
    }
    fn rrc_to_e(&mut self, op: u8) {
        self.e = self.rrc_r(op);
        self.save_op("RRC (IX + d) E");
    }
    fn rrc_to_h(&mut self, op: u8) {
        self.h = self.rrc_r(op);
        self.save_op("RRC (IX + d) H");
    }
    fn rrc_to_l(&mut self, op: u8) {
        self.l = self.rrc_r(op);
        self.save_op("RRC (IX + d) L");
    }
    fn rrc_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.rrc_r(op);
        mem.poke(addr, new_op);
        self.save_op("RRC (IX + d)");
    }
    fn rrc_to_a(&mut self, op: u8) {
        self.a = self.rrc_r(op);
        self.save_op("RRC (IX + d) A");
    }
    fn rl_to_b(&mut self, op: u8) {
        self.b = self.rl_r(op);
        self.save_op("RL (IX + d) B");
    }
    fn rl_to_c(&mut self, op: u8) {
        self.c = self.rl_r(op);
        self.save_op("RL (IX + d) C");
    }
    fn rl_to_d(&mut self, op: u8) {
        self.d = self.rl_r(op);
        self.save_op("RL (IX + d) D");
    }
    fn rl_to_e(&mut self, op: u8) {
        self.e = self.rl_r(op);
        self.save_op("RL (IX + d) E");
    }
    fn rl_to_h(&mut self, op: u8) {
        self.h = self.rl_r(op);
        self.save_op("RL (IX + d) H");
    }
    fn rl_to_l(&mut self, op: u8) {
        self.l = self.rl_r(op);
        self.save_op("RL (IX + d) L");
    }
    fn rl_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.rl_r(op);
        mem.poke(addr, new_op);
        self.save_op("RL (IX + d)")
    }
    fn rl_to_a(&mut self, op: u8) {
        self.a = self.rl_r(op);
        self.save_op("RL (IX + d) A");
    }
    fn rr_to_b(&mut self, op: u8) {
        self.b = self.rr_r(op);
        self.save_op("RR (IX + d) B");
    }
    fn rr_to_c(&mut self, op: u8) {
        self.c = self.rr_r(op);
        self.save_op("RR (IX + d) C");
    }
    fn rr_to_d(&mut self, op: u8) {
        self.d = self.rr_r(op);
        self.save_op("RR (IX + d) D");
    }
    fn rr_to_e(&mut self, op: u8) {
        self.e = self.rr_r(op);
        self.save_op("RR (IX + d) E");
    }
    fn rr_to_h(&mut self, op: u8) {
        self.h = self.rr_r(op);
        self.save_op("RR (IX + d) H");
    }
    fn rr_to_l(&mut self, op: u8) {
        self.l = self.rr_r(op);
        self.save_op("RR (IX + d) L");
    }
    fn rr_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.rr_r(op);
        mem.poke(addr, new_op);
        self.save_op("RR (IX + d)");
    }
    fn rr_to_a(&mut self, op: u8) {
        self.a = self.rr_r(op);
        self.save_op("RR (IX + d) A");
    }
    fn sla_to_b(&mut self, op: u8) {
        self.b = self.sla_r(op);
        self.save_op("SLA (IX + d) B");
    }
    fn sla_to_c(&mut self, op: u8) {
        self.c = self.sla_r(op);
        self.save_op("SLA (IX + d) C");
    }
    fn sla_to_d(&mut self, op: u8) {
        self.d = self.sla_r(op);
        self.save_op("SLA (IX + d) D");
    }
    fn sla_to_e(&mut self, op: u8) {
        self.e = self.sla_r(op);
        self.save_op("SLA (IX + d) E");
    }
    fn sla_to_h(&mut self, op: u8) {
        self.h = self.sla_r(op);
        self.save_op("SLA (IX + d) H");
    }
    fn sla_to_l(&mut self, op: u8) {
        self.l = self.sla_r(op);
        self.save_op("SLA (IX + d) L");
    }
    fn sla_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.sla_r(op);
        mem.poke(addr, new_op);
        self.save_op("SLA (IX + d)");
    }
    fn sla_to_a(&mut self, op: u8) {
        self.a = self.sla_r(op);
        self.save_op("SLA (IX + d) A");
    }
    fn sra_to_b(&mut self, op: u8) {
        self.b = self.sra_r(op);
        self.save_op("SRA (IX + d) B");
    }
    fn sra_to_c(&mut self, op: u8) {
        self.c = self.sra_r(op);
        self.save_op("SRA (IX + d) C");
    }
    fn sra_to_d(&mut self, op: u8) {
        self.d = self.sra_r(op);
        self.save_op("SRA (IX + d) D");
    }
    fn sra_to_e(&mut self, op: u8) {
        self.e = self.sra_r(op);
        self.save_op("SRA (IX + d) E");
    }
    fn sra_to_h(&mut self, op: u8) {
        self.h = self.sra_r(op);
        self.save_op("SRA (IX + d) H");
    }
    fn sra_to_l(&mut self, op: u8) {
        self.l = self.sra_r(op);
        self.save_op("SRA (IX + d) L");
    }
    fn sra_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.sra_r(op);
        mem.poke(addr, new_op);
        self.save_op("SRA (IX + d)");
    }
    fn sra_to_a(&mut self, op: u8) {
        self.a = self.sra_r(op);
        self.save_op("SRA (IX + d) A");
    }
    fn sll_to_b(&mut self, op: u8) {
        self.b = self.sll_r(op);
        self.save_op("SLL (IX + d) B");
    }
    fn sll_to_c(&mut self, op: u8) {
        self.c = self.sll_r(op);
        self.save_op("SLL (IX + d) C");
    }
    fn sll_to_d(&mut self, op: u8) {
        self.d = self.sll_r(op);
        self.save_op("SLL (IX + d) D");
    }
    fn sll_to_e(&mut self, op: u8) {
        self.e = self.sll_r(op);
        self.save_op("SLL (IX + d) E");
    }
    fn sll_to_h(&mut self, op: u8) {
        self.h = self.sll_r(op);
        self.save_op("SLL (IX + d) H");
    }
    fn sll_to_l(&mut self, op: u8) {
        self.l = self.sll_r(op);
        self.save_op("SLL (IX + d) L");
    }
    fn sll_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.sll_r(op);
        mem.poke(addr, new_op);
        self.save_op("SLL (IX + d)");
    }
    fn sll_to_a(&mut self, op: u8) {
        self.a = self.sll_r(op);
        self.save_op("SLL (IX + d) A");
    }
    fn srl_to_b(&mut self, op: u8) {
        self.b = self.srl_r(op);
        self.save_op("SLL (IX + d) B");
    }
    fn srl_to_c(&mut self, op: u8) {
        self.c = self.srl_r(op);
        self.save_op("SLL (IX + d) C");
    }
    fn srl_to_d(&mut self, op: u8) {
        self.d = self.srl_r(op);
        self.save_op("SLL (IX + d) D");
    }
    fn srl_to_e(&mut self, op: u8) {
        self.e = self.srl_r(op);
        self.save_op("SLL (IX + d) E");
    }
    fn srl_to_h(&mut self, op: u8) {
        self.h = self.srl_r(op);
        self.save_op("SLL (IX + d) H");
    }
    fn srl_to_l(&mut self, op: u8) {
        self.l = self.sll_r(op);
        self.save_op("SLL (IX + d) L");
    }
    fn srl_at_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = self.srl_r(op);
        mem.poke(addr, new_op);
        self.save_op("SRL (IX + d)");
    }
    fn srl_to_a(&mut self, op: u8) {
        self.a = self.srl_r(op);
        self.save_op("SLL (IX + d) A");
    }
    fn bit_0_ixy(&mut self, op: u8) {
        self.bit_n_r(0, op);
        self.save_op("BIT 0 (IX + d)")
    }
    fn bit_1_ixy(&mut self, op: u8) {
        self.bit_n_r(1, op);
        self.save_op("BIT 1 (IX + d)")
    }
    fn bit_2_ixy(&mut self, op: u8) {
        self.bit_n_r(2, op);
        self.save_op("BIT 2 (IX + d)")
    }
    fn bit_3_ixy(&mut self, op: u8) {
        self.bit_n_r(3, op);
        self.save_op("BIT 3 (IX + d)")
    }
    fn bit_4_ixy(&mut self, op: u8) {
        self.bit_n_r(4, op);
        self.save_op("BIT 4 (IX + d)")
    }
    fn bit_5_ixy(&mut self, op: u8) {
        self.bit_n_r(5, op);
        self.save_op("BIT 5 (IX + d)")
    }
    fn bit_6_ixy(&mut self, op: u8) {
        self.bit_n_r(6, op);
        self.save_op("BIT 6 (IX + d)")
    }
    fn bit_7_ixy(&mut self, op: u8) {
        self.bit_n_r(7, op);
        self.save_op("BIT 7 (IX + d)")
    }
    fn res_0_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(0, op);
        mem.poke(addr, new_op);
        self.save_op("res 0 (IX + d)");
    }
    fn res_1_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(1, op);
        mem.poke(addr, new_op);
        self.save_op("res 1 (IX + d)");
    }
    fn res_2_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(2, op);
        mem.poke(addr, new_op);
        self.save_op("res 2 (IX + d)");
    }
    fn res_3_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(3, op);
        mem.poke(addr, new_op);
        self.save_op("res 3 (IX + d)");
    }
    fn res_4_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(4, op);
        mem.poke(addr, new_op);
        self.save_op("res 4 (IX + d)");
    }
    fn res_5_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(5, op);
        mem.poke(addr, new_op);
        self.save_op("res 5 (IX + d)");
    }
    fn res_6_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(6, op);
        mem.poke(addr, new_op);
        self.save_op("res 6 (IX + d)");
    }
    fn res_7_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::res_n_r(7, op);
        mem.poke(addr, new_op);
        self.save_op("res 7 (IX + d)");
    }
    fn set_0_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(0, op);
        mem.poke(addr, new_op);
        self.save_op("set 0 (IX + d)");
    }
    fn set_1_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(1, op);
        mem.poke(addr, new_op);
        self.save_op("set 1 (IX + d)");
    }
    fn set_2_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(2, op);
        mem.poke(addr, new_op);
        self.save_op("set 2 (IX + d)");
    }
    fn set_3_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(3, op);
        mem.poke(addr, new_op);
        self.save_op("set 3 (IX + d)");
    }
    fn set_4_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(4, op);
        mem.poke(addr, new_op);
        self.save_op("set 4 (IX + d)");
    }
    fn set_5_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(5, op);
        mem.poke(addr, new_op);
        self.save_op("set 5 (IX + d)");
    }
    fn set_6_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(6, op);
        mem.poke(addr, new_op);
        self.save_op("set 6 (IX + d)");
    }
    fn set_7_ixy(&mut self, mem: &mut Memory, addr: u16, op: u8) {
        let new_op = Z80::set_n_r(7, op);
        mem.poke(addr, new_op);
        self.save_op("set 7 (IX + d)");
    }
    fn lddr(&mut self, mem: &mut Memory) {
        let mut source_addr = Z80::get_word(self.h, self.l);
        let mut dest_addr = Z80::get_word(self.d, self.e);
        let mut counter = Z80::get_word(self.b, self.c);
        let val = mem.peek(source_addr);
        mem.poke(dest_addr, val);

        source_addr = source_addr.wrapping_sub(1);
        let new_h_l = Z80::get_bytes(source_addr);
        self.h = new_h_l.0;
        self.l = new_h_l.1;
        dest_addr = dest_addr.wrapping_sub(1);
        let new_h_l = Z80::get_bytes(dest_addr);
        self.d = new_h_l.0;
        self.e = new_h_l.1;
        counter = counter.wrapping_sub(1);
        let new_h_l = Z80::get_bytes(counter);
        self.b = new_h_l.0;
        self.c = new_h_l.1;
        if counter > 0 {
            self.pc = self.pc.wrapping_sub(2);
        }
        self.save_op("LDDR");
    }
    fn neg(&mut self) {
        let old_a = self.a;
        let new_a = !old_a;
        self.a = new_a;
        self.set_reset_flag((new_a as i8) < 0, S);
        self.set_reset_flag(new_a == 0, Z);
        self.set_reset_flag((new_a & 0xf) < (old_a & 0xf), H);
        self.set_reset_flag(old_a == 0x80, P_V);
        self.set_flag(N);
        self.set_reset_flag(old_a != 0x00, C);
    }
    fn retn(&mut self, mem: &Memory) {
        let lo = mem.peek(self.sp);
        let hi = mem.peek(self.sp + 1);
        self.pc = Z80::get_word(hi, lo);
        self.sp += 2;
    }
    fn rrd(&mut self) {
        let old_a = self.a & 0xf;
        self.a = self.l;
        self.h = old_a;
    }
}
