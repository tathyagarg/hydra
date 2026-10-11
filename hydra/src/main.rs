pub mod cpu;
pub mod memory;

fn main() {
    let mut cpu = cpu::Cpu::default();
    let mut memory = memory::Memory::init(
        vec![
            0x00, 0x00, 0x80, 0xD2, // MOVZ X0, #0
            0x21, 0x00, 0x80, 0xD2, // MOVZ X1, #1
            0x20, 0x00, 0x00, 0x8B, // ADD X0, X0, X1
            0x21, 0x04, 0x00, 0x91, // ADD X1, X1, #1
            0x3F, 0x94, 0x01, 0xF1, // CMP X1, #101
            0x8B, 0xFF, 0xFF, 0x54, // B.LT -3
        ],
        memory::Endian::Little,
    );

    loop {
        println!("PC: {:?}", cpu.get_PC64());
        if cpu.step(&mut memory).is_err() {
            break;
        }
    }

    println!("X0: {:?}", cpu.__dbg_get_x(0));
    println!("X1: {:?}", cpu.__dbg_get_x(1));
}
