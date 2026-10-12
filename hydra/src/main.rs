pub mod cpu;
pub mod memory;

fn main() {
    let mut cpu = cpu::Cpu::default();
    let mut memory = memory::Memory::init(
        vec![
            0x42, 0xD6, 0x8B, 0xD2, // MOVZ X2, #24242
            0x00, 0x00, 0x80, 0xD2, // MOVZ X0, #0
            0x21, 0x00, 0x80, 0xD2, // MOVZ X1, #1
            0x20, 0x00, 0x00, 0x8B, // ADD X0, X0, X1
            0x21, 0x04, 0x00, 0x91, // ADD X1, X1, #1
            0x5F, 0x00, 0x01, 0xEB, // CMP X1, X2
            0x81, 0xFF, 0xFF, 0x54, // B.NE -3
        ],
        memory::Endian::Little,
    );

    loop {
        if cpu.step(&mut memory).is_err() {
            break;
        }
    }

    println!("Sum is: {:?}", cpu.__dbg_get_x(0));
}
