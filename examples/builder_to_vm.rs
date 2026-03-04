use alphabetvm::{ImageBuilder, Vm, is::inst};

fn main() {
    const LEN_ADDR: u32 = 0x30;
    const ARRAY_ADDR: u32 = 0x31;
    const R_ARRAY_PTR: usize = 1;
    const R_LEN: usize = 2;
    const R_SUM: usize = 3;
    const R_ARRAY_ELEM: usize = 4;

    let array = vec![4u8, 7u8, -2i8 as u8];
    let len = array.len() as u8;

    // Create a program that reads signed bytes
    // from an array and calculates their sum.
    let instructions = &[
        inst::ldbu(R_LEN, 0, LEN_ADDR as i16),
        inst::addi(R_ARRAY_PTR, 0, ARRAY_ADDR as u16),
        inst::add(R_SUM, 0, 0),
        inst::beq(R_LEN, 0, 6),
        inst::ldb(R_ARRAY_ELEM, R_ARRAY_PTR, 0),
        inst::add(R_SUM, R_SUM, R_ARRAY_ELEM),
        inst::addi(R_ARRAY_PTR, R_ARRAY_PTR, 1),
        inst::subi(R_LEN, R_LEN, 1),
        inst::jmp(0, -5),
        inst::jmp(0, 0),
    ];
    let builder = ImageBuilder::new()
        .write_instructions(instructions)
        .seek(LEN_ADDR.into())
        .write_byte(len)
        .seek(ARRAY_ADDR.into())
        .write_bytes(array);
    let mut vm: Vm = builder.build().expect("failed to build VM");

    // Execute until the last instruction is reached.
    _ = vm.run_until_loop();

    // Print the sum.
    let sum = vm.register(R_SUM);
    println!("Sum: {sum}");
}
