mod memory;
mod z80;
extern crate minifb;

use minifb::{Key, WindowOptions, Window};

const WIDTH: usize = 256;
const HEIGHT: usize = 192;

// const C : u8 = 1;
// const N : u8 = 2;
// const P_V: u8 = 4;
// const BIT_3: u8 = 8;
// const H: u8 = 16;
// const BIT_5: u8 = 32;
// const Z: u8 = 64;
// const S: u8  = 128;
fn get_color(color: u8) -> u32 {
    let x = match color {
        0 => 0x000000,
        1 => 0x0000D7,
        2 => 0xD70000,
        3 => 0xD700D7,
        4 => 0x00D700,
        5 => 0x00D7D7,
        6 => 0xD7D700,
        7 => 0xD7D7D7,
        _ => panic!("Color no definido"),
    };

    return x;
}
fn get_color_for_byte(mem: &memory::Memory, address: u16) -> (u32, u32) {
    let relative_address = address - 0x4000;
    let attribute_relative_address = (relative_address >> 3) & 0x300 | relative_address & 0xFF;
    let contents = mem.peek(attribute_relative_address + 0x5800);
    let foreground = get_color(contents & 0x07);
    let background = get_color((contents >> 3) & 0x0F);
    (foreground, background)
}

fn print_screen(mem: &memory::Memory) {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new("Test - ESC to exit",
                                 WIDTH,
                                 HEIGHT,
                                 WindowOptions::default()).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Base bitmat video memory
    let init_video_mem = 0x4000;
    let end_video_mem = 0x57FF;
    for addr in init_video_mem..end_video_mem {
        let pixel = mem.peek(addr);
        let colors = get_color_for_byte(mem, addr);
        let foreground = colors.0;
        let background = colors.1;
        // The memory address to coordinates
        // has the format
        //       H
        // 0 1 0 Y7 Y6 Y2 Y1 Y0 Y5
        //       L
        // Y4 Y3 X4 X3 X2 X1 X0
        let x_in_byte = addr & 0x1F;
        let y = (addr & 0x1800 | addr & 0x700 >> 3 | addr & 0x14 << 3) >> 5;

        for i in 0..7 {
            let x_in_bit = x_in_byte << 3 | i;
            let mask = 0x80 >> i;
            let index: usize = (y as usize * WIDTH) + x_in_bit as usize;
            if pixel & mask != 0 {
                //set_pixel(x_in_bit, y, foreground);
                buffer[index] = foreground as u32;
            } else {
                //set_pixel(x_in_bit, y, background);
                buffer[index] = background as u32;
            }
        }
        while window.is_open() && !window.is_key_down(Key::Escape) {
          window.update_with_buffer(&buffer).unwrap();
        }
    }
}

fn main() {
    let mut micro = z80::Z80::new();
    let mut mem = memory::Memory::new();

    while !micro.halt {
        micro.exec(&mut mem);
    }

    print_screen(&mem);
}
