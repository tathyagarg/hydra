pub mod cpu;
pub mod memory;

fn main() {
    let mut cpu = cpu::Cpu::default();
    let mut memory = memory::Memory::from(vec![
        0xd2, 0x80, 0x00, 0x61, // MOVZ X1, #3
        0xd2, 0x80, 0x00, 0x42, // MOVZ X2, #2
        0x8b, 0x01, 0x00, 0x43, // ADD X3, X1, X2
        0xcb, 0x02, 0x00, 0x24, // SUB X4, X1, X2
    ]);

    for _ in 0..4 {
        cpu.step(&mut memory).unwrap();
    }

    println!("X1: {:?}", cpu.__dbg_get_x(1));
    println!("X2: {:?}", cpu.__dbg_get_x(2));
    println!("X3: {:?}", cpu.__dbg_get_x(3));
    println!("X4: {:?}", cpu.__dbg_get_x(4));
}
