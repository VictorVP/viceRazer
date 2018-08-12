use memory::*;

const C : u8 = 1;
const N : u8 = 2;
const P_V: u8 = 4;
const BIT_3: u8 = 8;
const H: u8 = 16;
const BIT_5: u8 = 32;
const Z: u8 = 64;
const S: u8  = 128;

pub struct z80 {
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
    iff2: bool    
}

impl z80 {
    pub fn new() -> z80 {
        let out = z80 {
            halt: false,
            pc: 0,
            sp: 0,
            ix_h: 0,
            ix_l: 0,
            iy_h: 0,
            iy_l: 0,
            i: 0,
            r: 0,
            a: 0,
            f: 0,
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
            iff2: false
        };

        out
    }
    
    pub fn set_af (&mut self, value:u16) {        
        let hi = (value >> 8) as u8;
        self.a = hi;
        self.f = (value & 127) as u8;
    }

    pub fn get_af(&self) -> u16 {
        let a_aux = self.a as u16;

        ((a_aux << 8) as u16 + (self.f) as u16)
    }   

    fn check_flag(flag_container: u8, flag: u8) -> bool {
        flag_container & flag != 0
    }

    fn set_flag(flag_container: u8, flag: u8, value: bool) -> u8 {       
        if value {
            flag_container | flag
        } else {        
            let complement = if flag == 128 {
                1
            } else {
                127 - flag
            };
            flag_container & complement
        }
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

    pub fn exec(&mut self, mem: &mut memory) {
        let byte = self.read_bus(mem);

        // Tomar los tres últimos bits 
        // para posible identificación de registro
        let lo = byte & 7;

        match byte {
            0x00 => z80::nop(),
            0x01 => self.ld_bc(mem),
            0x06 => self.ld_b_n(mem),
            0x0E => self.ld_c_n(mem),
            0x11 => self.ld_de(mem),
            0x16 => self.ld_d_n(mem),
            0x1E => self.ld_e_n(mem),
            0x20 => self.jr_nz_e(mem),
            0x21 => self.ld_hl(mem),
            0x26 => self.ld_h_n(mem),
            0x2B => self.dec_hl(),
            0x2E => self.ld_l_n(mem),
            0x31 => self.ld_sp(mem),
            0x36 => self.ld_hl_n(mem),
            0x3E => self.ld_a_n(mem),
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

            0xAF => self.xor_a(lo),
            0xBC => self.cp_h(),
            0xC3 => self.jp_nn(mem),
            0xD3 => self.out_n_a(mem),
            0xED => {
                let byte2 = self.read_bus(mem);
                self.decode_ed_instructions(mem, byte2);
            },
            0xF3 => self.di(),
                _ => {println!("El opCode {:x} no está implementado", byte);
                self.halt = true;
            },
       }
    }

    fn read_bus(&mut self, mem: &memory) -> u8 {
        let res = mem.peek(self.pc);
        print!("{:x} {:x}     ", self.pc, res);
        self.pc += 1;
        res
    }

    fn nop() {
        println!("NOP");
    }
    fn di(&mut self) {
        println!("DI");
        self.iff1 = false;
        self.iff2 = false;
    }

    fn xor_a(&mut self, ident: u8) {
        let value = match ident {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.f, //no debería pasar!!
            7 => self.a,
            _ => 0,
        };

        println!("XOR A {}", value);
        self.a = self.a ^ value;
        z80::set_flag(self.f, S, self.a & 0x80 > 0);
        z80::set_flag(self.f, Z, self.a == 0);
        z80::set_flag(self.f, P_V, z80::check_byte_parity(self.a));
    }

    fn ld_bc(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        self.c = x1;
        self.b = x2;
        let dir = ((x1 as u16) << 8) + (x2 as u16);
        println!("LD BC {:x}", dir);
    }

    fn ld_de(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        self.d = x1;
        self.e = x2;
        let dir = ((x1 as u16) << 8) + (x2 as u16);
        println!("LD DE {:x}", dir);
    }

    fn jr_nz_e(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem) as i8;
        println!("Leído offset {} para {}", x1, self.pc);
        let x2 = self.pc as i16 + x1 as i16;
        println!("Nueva dirección {} -> {:x} (Se convierte a {:x})", x2, x2, x2 as u16);
        println!("La condición de salto es: {}", z80::check_flag(self.f,Z));
        if z80::check_flag(self.f, Z) {
            
            self.pc = x2 as u16;
        }

        println!("JR NZ {:x}    PC {:x}", x1, self.pc);
    }

    fn ld_hl(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        self.h = x1;
        self.l = x2;
        let dir = ((x1 as u16) << 8) + (x2 as u16);
        println!("LD HL {:x}", dir);
    }

    fn ld_sp(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        let dir = ((x1 as u16) << 8) + (x2 as u16);
        self.sp = dir;        
        
        println!("LD SP {:x} {:x}", x1, x1);
    }

    fn ld_hl_n(&mut self, mem: &mut memory) {
        let address = ((self.h as u16) << 8) + (self.l as u16);
        let n = self.read_bus(mem);
        mem.poke(address, n);

        println!("LD HL {:x}", n);
    }
    fn jp_nn(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        let x2 = self.read_bus(mem);
        let dir = ((x2 as u16) << 8) + (x1 as u16);

        self.pc = dir;
        println!("JP {:x} {:x}", x2, x1);
     }                                                                      

    fn ld_b_b(&mut self)
    {
        self.b = self.b;
        println!("LD B B");
    }

    fn ld_b_c(&mut self)
    {
        self.b = self.c;
        println!("LD B C");
    }

    fn ld_b_d(&mut self)
    {
        self.b = self.d;
        println!("LD B D");
    }
    fn ld_b_e(&mut self)
    {
        self.b = self.e;
        println!("LD B E");
    }
    fn ld_b_h(&mut self)
    {
        self.b = self.h;
        println!("LD B H");
    }
    fn ld_b_l(&mut self)
    {
        self.b = self.l;
        println!("LD B L");
    }
    fn ld_b_hl(&mut self, mem: &memory)
    {
        let addr = (self.h as u16) << 8 + (self.l as u16);
        let byte = mem.peek(addr);
        self.b = byte;
        println!("LD B HL");
    }
    fn ld_b_a(&mut self)
    {
        self.b = self.a;
        println!("LD B A");
    }                            
    fn ld_c_b(&mut self)
    {
        self.c = self.b;
        println!("LD C B");
    }

    fn ld_c_c(&mut self)
    {
        self.c = self.c;
        println!("LD C C");
    }

    fn ld_c_d(&mut self)
    {
        self.c = self.d;
        println!("LD C D");
    }
    fn ld_c_e(&mut self)
    {
        self.c = self.e;
        println!("LD C E");
    }
    fn ld_c_h(&mut self)
    {
        self.c = self.h;
        println!("LD C H");
    }
    fn ld_c_l(&mut self)
    {
        self.c = self.l;
        println!("LD C L");
    }
    fn ld_c_hl(&mut self, mem: &memory)
    {
        let addr = (self.h as u16) << 8 + (self.l as u16);
        let byte = mem.peek(addr);
        self.c = byte;
        println!("LD C HL");
    }
    fn ld_c_a(&mut self)
    {
        self.c = self.a;
        println!("LD C A");
    }
    fn ld_d_b(&mut self)
    {
        self.d = self.b;
        println!("LD D B");
    }

    fn ld_d_c(&mut self)
    {
        self.d = self.c;
        println!("LD D C");
    }

    fn ld_d_d(&mut self)
    {
        self.d = self.d;
        println!("LD D D");
    }
    fn ld_d_e(&mut self)
    {
        self.d = self.e;
        println!("LD D E");
    }
    fn ld_d_h(&mut self)
    {
        self.d = self.h;
        println!("LD D H");
    }
    fn ld_d_l(&mut self)
    {
        self.d = self.l;
        println!("LD D L");
    }
    fn ld_d_hl(&mut self, mem: &memory)
    {
        let addr = (self.h as u16) << 8 + (self.l as u16);
        let byte = mem.peek(addr);
        self.d = byte;
        println!("LD D HL");
    }
    fn ld_d_a(&mut self)
    {
        self.d = self.a;
        println!("LD D A");
    }
    fn ld_e_b(&mut self)
    {
        self.e = self.b;
        println!("LD E B");
    }

    fn ld_e_c(&mut self)
    {
        self.e = self.c;
        println!("LD E C");
    }

    fn ld_e_d(&mut self)
    {
        self.e = self.d;
        println!("LD E D");
    }
    fn ld_e_e(&mut self)
    {
        self.e = self.e;
        println!("LD E E");
    }
    fn ld_e_h(&mut self)
    {
        self.e = self.h;
        println!("LD E H");
    }
    fn ld_e_l(&mut self)
    {
        self.e = self.l;
        println!("LD E L");
    }
    fn ld_e_hl(&mut self, mem: &memory)
    {
        let addr = (self.h as u16) << 8 + (self.l as u16);
        let byte = mem.peek(addr);
        self.e = byte;
        println!("LD E HL");
    }
    fn ld_e_a(&mut self)
    {
        self.e = self.a;
        println!("LD E A");
    }

    fn ld_h_b(&mut self)
    {
        self.h = self.b;
        println!("LD H B");
    }

    fn ld_h_c(&mut self)
    {
        self.h = self.c;
        println!("LD H C");
    }

    fn ld_h_d(&mut self)
    {
        self.h = self.d;
        println!("LD H D");
    }
    fn ld_h_e(&mut self)
    {
        self.h = self.e;
        println!("LD H E");
    }
    fn ld_h_h(&mut self)
    {
        self.h = self.h;
        println!("LD H H");
    }
    fn ld_h_l(&mut self)
    {
        self.h = self.l;
        println!("LD H L");
    }
    fn ld_h_hl(&mut self, mem: &memory)
    {
        let addr = (self.h as u16) << 8 + (self.l as u16);
        let byte = mem.peek(addr);
        self.h = byte;
        println!("LD H HL");
    }
    fn ld_h_a(&mut self)
    {
        self.h = self.a;
        println!("LD H A");
    }
            
    fn ld_l_b(&mut self)
    {
        self.l = self.b;
        println!("LD L B");
    }

    fn ld_l_c(&mut self)
    {
        self.l = self.c;
        println!("LD L C");
    }

    fn ld_l_d(&mut self)
    {
        self.l = self.d;
        println!("LD L D");
    }
    fn ld_l_e(&mut self)
    {
        self.l = self.e;
        println!("LD L E");
    }
    fn ld_l_h(&mut self)
    {
        self.l = self.h;
        println!("LD L H");
    }
    fn ld_l_l(&mut self)
    {
        self.l = self.l;
        println!("LD L L");
    }
    fn ld_l_hl(&mut self, mem: &memory)
    {
        let addr = (self.h as u16) << 8 + (self.l as u16);
        let byte = mem.peek(addr);
        self.l = byte;
        println!("LD L HL");
    }
    fn ld_l_a(&mut self)
    {
        self.l = self.a;
        println!("LD L A");
    }

    fn ld_a_n(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        self.a = x1;

        println!("LD A {:x}", x1);
    }

    fn ld_b_n(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        self.b = x1;

        println!("LD B {:x}", x1);
    }

    fn ld_c_n(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        self.c = x1;

        println!("LD C {:x}", x1);
    }

    fn ld_d_n(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        self.d = x1;

        println!("LD D {:x}", x1);
    }

    fn ld_e_n(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        self.e = x1;

        println!("LD E {:x}", x1);
    }

    fn ld_h_n(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        self.h = x1;

        println!("LD H {:x}", x1);
    }

    fn cp_h(&mut self) {
        println!("Voy a comparar A {:x} con H {:x}", self.a, self.h);
        z80::set_flag(self.f, Z, self.a == self.h);
        z80::set_flag(self.f, N, self.a < self.h);

        println!("CP H");
    }

    fn dec_hl (&mut self) {
        let mut val = ((self.h as u16) << 8) + (self.l as u16);        
        val -= 1;
        self.h = (val >> 8) as u8;
        self.l = (val & 127) as u8;

        println!("DEC HL");
    }

    fn ld_l_n(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        self.l = x1;

        println!("LD H {:x}", x1);
    }

    
    fn out_n_a(&mut self, mem: &memory) {
        let x1 = self.read_bus(mem);
        let dir = ((self.a as u16) << 8) + (x1 as u16);
        println!("out {:x} A", x1);
        println!("  TODO: poner el valor {} en la dirección {:x}", self.a, dir);
    }

    fn decode_ed_instructions(&mut self, mem: &memory, byte: u8){
        match byte {
            0x47 => self.ld_i_a(),
            _ => println!("El opCode ed {:x} no está implementado", byte),
        }
    }

    fn ld_i_a(&mut self) {
        self.i = self.a;
        println!("LD I A");
    }
}