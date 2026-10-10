pub mod cpu;
pub mod memory;

fn main() {
    let mut cpu = cpu::Cpu::default();
    let mut memory = memory::Memory::init(
        vec![
            0x61, 0x00, 0x80, 0xd2, // MOVZ X1, #3
            0x42, 0x00, 0x80, 0xd2, // MOVZ X2, #2
            0x43, 0x00, 0x01, 0x8b, // ADD X3, X1, X2
            0x24, 0x00, 0x02, 0xcb, // SUB X4, X1, X2
        ],
        memory::Endian::Little,
    );

    for _ in 0..4 {
        cpu.step(&mut memory).unwrap();
    }

    println!("X1: {:?}", cpu.__dbg_get_x(1));
    println!("X2: {:?}", cpu.__dbg_get_x(2));
    println!("X3: {:?}", cpu.__dbg_get_x(3));
    println!("X4: {:?}", cpu.__dbg_get_x(4));
}
