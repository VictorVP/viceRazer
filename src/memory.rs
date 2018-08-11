pub const ROM_48K: &'static [u8; 16 * 1024] = include_bytes!("48.rom");
pub struct memory {
    // Simular una memoria de 64 K
    contents: [u8; 64 * 1024]
}

impl memory {
    pub fn new () -> memory {
        let mut out = memory {
         contents: [0; 64 * 1024]
        };

        out.load_rom(ROM_48K);

        out
    }
    pub fn peek (&self, addr: u16) -> u8 {
        self.contents[addr as usize]
    }

    pub fn poke (&mut self, addr: u16, value: u8) {
        self.contents[addr as usize] = value;
    }

    pub fn load_rom(&mut self, rom: &[u8])
    {        
        self.contents[..16 * 1024].copy_from_slice(&rom);        
    }
}