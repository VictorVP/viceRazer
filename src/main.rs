mod z80;
mod memory;

const C : u8 = 1;
const N : u8 = 2;
const P_V: u8 = 4;
const BIT_3: u8 = 8;
const H: u8 = 16;
const BIT_5: u8 = 32;
const Z: u8 = 64;
const S: u8  = 128;

fn main() {
    let mut micro = z80::z80::new();
    let mut mem = memory::memory::new();

    while !micro.halt {
       micro.exec(&mut mem);
    }
}