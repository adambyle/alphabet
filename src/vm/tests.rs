use super::*;

#[test]
pub fn byte_address_rounding() {
    // Round down.
    assert_eq!(ByteAddress::from(0).word_address_round_down().value(), 0);
    assert_eq!(ByteAddress::from(1).word_address_round_down().value(), 0);
    assert_eq!(ByteAddress::from(2).word_address_round_down().value(), 0);
    assert_eq!(ByteAddress::from(3).word_address_round_down().value(), 0);
    assert_eq!(ByteAddress::from(4).word_address_round_down().value(), 1);

    // Round up.
    let tests = [(0, 0), (1, 1), (2, 1), (3, 1), (4, 1)];
    for (byte_addr, word_addr_expected) in tests {
        let (word_addr, overflow) = ByteAddress::from(byte_addr).word_address_round_up();
        assert!(!overflow);
        assert_eq!(word_addr.value(), word_addr_expected);
    }

    // Round up overflow.
    for byte_addr in (u32::MAX - 2)..=(u32::MAX) {
        let (word_addr, overflow) = ByteAddress::from(byte_addr).word_address_round_up();
        assert!(overflow);
        assert_eq!(word_addr.value(), 0);
    }
}

#[test]
pub fn word_address_ops() {
    let word_addr_max = 0x3FFFFFFF;

    // Basic unsigned addition.
    let zero_addr = WordAddress::from(0);
    for offset in 1..=10 {
        let (sum, overflow) = zero_addr.overflowing_add(offset);
        assert!(!overflow);
        assert_eq!(sum.value(), offset);
    }
    let base = 0x3AE7;
    let base_addr = WordAddress::from(base);
    for offset in 1..=10 {
        let (sum, overflow) = base_addr.overflowing_add(offset);
        assert!(!overflow);
        assert_eq!(sum.value(), base + offset);
    }

    // Overflowing addition.
    let base = word_addr_max - 5;
    let base_addr = WordAddress::from(base);
    for offset in 1..=5 {
        let (sum, overflow) = base_addr.overflowing_add(offset);
        assert!(!overflow);
        assert_eq!(sum.value(), base + offset);
    }
    for offset in 6..=10 {
        let (sum, overflow) = base_addr.overflowing_add(offset);
        assert!(overflow);
        assert_eq!(sum.value(), offset - 6);
    }

    // Signed addition.
    let zero_addr = WordAddress::from(0);
    for offset in 1..=10 {
        let (sum, overflow) = zero_addr.overflowing_add_signed(offset);
        assert!(!overflow);
        assert_eq!(sum.value(), offset as u32);
    }
    let base = 0x3AE7;
    let base_addr = WordAddress::from(base);
    for offset in 1..=10 {
        let (sum, overflow) = base_addr.overflowing_add_signed(offset);
        assert!(!overflow);
        assert_eq!(sum.value(), base + offset as u32);
    }

    // Negative signed addition (equivalent to subtraction).
    for offset in 1..=10 {
        let (sum, overflow) = base_addr.overflowing_add_signed(-offset);
        assert!(!overflow);
        assert_eq!(sum.value(), base - offset as u32);
    }

    // Overflowing positive signed addition.
    let base = word_addr_max - 5;
    let base_addr = WordAddress::from(base);
    for offset in 1..=5 {
        let (sum, overflow) = base_addr.overflowing_add_signed(offset);
        assert!(!overflow);
        assert_eq!(sum.value(), base + offset as u32);
    }
    for offset in 6..=10 {
        let (sum, overflow) = base_addr.overflowing_add_signed(offset);
        assert!(overflow);
        assert_eq!(sum.value(), offset as u32 - 6);
    }

    // Overflowing negative signed addition (subtract past 0).
    let base = 5;
    let base_addr = WordAddress::from(base);
    for offset in 1..=5 {
        let (sum, overflow) = base_addr.overflowing_add_signed(-offset);
        assert!(!overflow);
        assert_eq!(sum.value(), base - offset as u32);
    }
    for offset in 6..=10 {
        let (sum, overflow) = base_addr.overflowing_add_signed(-offset);
        assert!(overflow);
        assert_eq!(sum.value(), word_addr_max - (offset as u32 - 6));
    }

    // Unsigned subtraction.
    let base = 0x3AE7;
    let base_addr = WordAddress::from(base);
    for offset in 1..=10 {
        let (diff, overflow) = base_addr.overflowing_sub(offset);
        assert!(!overflow);
        assert_eq!(diff.value(), base - offset);
    }

    // Subtracting to zero.
    let base_addr = WordAddress::from(5);
    for offset in 1..=5 {
        let (diff, overflow) = base_addr.overflowing_sub(offset);
        assert!(!overflow);
        assert_eq!(diff.value(), 5 - offset);
    }

    // Overflowing unsigned subtraction (subtract past 0).
    let base = 5;
    let base_addr = WordAddress::from(base);
    for offset in 6..=10 {
        let (diff, overflow) = base_addr.overflowing_sub(offset);
        assert!(overflow);
        assert_eq!(diff.value(), word_addr_max - (offset - 6));
    }

    // Signed subtraction.
    let base = 0x3AE7;
    let base_addr = WordAddress::from(base);
    for offset in 1..=10 {
        let (diff, overflow) = base_addr.overflowing_sub_signed(offset);
        assert!(!overflow);
        assert_eq!(diff.value(), base - offset as u32);
    }

    // Negative signed subtraction (equivalent to addition).
    for offset in 1..=10 {
        let (diff, overflow) = base_addr.overflowing_sub_signed(-offset);
        assert!(!overflow);
        assert_eq!(diff.value(), base + offset as u32);
    }

    // Overflowing signed subtraction past 0.
    let base = 5;
    let base_addr = WordAddress::from(base);
    for offset in 6..=10 {
        let (diff, overflow) = base_addr.overflowing_sub_signed(offset);
        assert!(overflow);
        assert_eq!(diff.value(), word_addr_max - (offset as u32 - 6));
    }

    // Overflowing negative signed subtraction (add past max).
    let base = word_addr_max - 5;
    let base_addr = WordAddress::from(base);
    for offset in 6..=10 {
        let (diff, overflow) = base_addr.overflowing_sub_signed(-offset);
        assert!(overflow);
        assert_eq!(diff.value(), offset as u32 - 6);
    }
}

#[test]
fn word_address_truncates_overflow() {
    // The top 2 bits should be masked off.
    assert_eq!(WordAddress::from(0x40000000).value(), 0);
    assert_eq!(WordAddress::from(0x80000000).value(), 0);
    assert_eq!(WordAddress::from(0xFFFFFFFF).value(), 0x3FFFFFFF);
    assert_eq!(WordAddress::from(0x7FFFFFFF).value(), 0x3FFFFFFF);
}

#[test]
fn block_index_base_addresses() {
    let tests = [
        (0x0000, 0x00000000, 0x00000000),
        (0x0001, 0x00010000, 0x00004000),
        (0x0002, 0x00020000, 0x00008000),
        (0x000F, 0x000F0000, 0x0003C000),
        (0x0100, 0x01000000, 0x00400000),
        (0xFFFF, 0xFFFF0000, 0x3FFFC000),
    ];
    for (index, byte_addr, word_addr) in tests {
        let block = BlockIndex::from(index);
        assert_eq!(block.base_byte_address().value(), byte_addr);
        assert_eq!(block.base_word_address().value(), word_addr);
    }
}

#[test]
fn block_index_base_addresses_match() {
    let indices = [0x0, 0x0001, 0x0010, 0x00FF, 0x1000, 0xFFFF];
    for index in indices {
        let block = BlockIndex::from(index);
        let from_byte = block.base_byte_address().word_address_round_down();
        let direct = block.base_word_address();
        assert_eq!(from_byte, direct, "inconsistency for index {index:#06X}");
    }
}

#[test]
fn block_index_add_offset() {
    let tests = [
        (0x0000_u16, 0x0000_u16),
        (0x0001, 0x0000),
        (0x0001, 0x0004),
        (0x0001, 0xFFFF),
        (0x00FF, 0x1234),
        (0xFFFF, 0xFFFF),
    ];
    for (index, offset) in tests {
        let block = BlockIndex::from(index);
        let off = BlockOffset::from(offset);
        let via_add = (block + off).value();
        let via_from = ByteAddress::from((block, off)).value();
        assert_eq!(
            via_add, via_from,
            "mismatch for index {index:#06X} offset {offset:#06X}",
        );
    }
}

#[test]
fn block_parts_round_trip() {
    let tests = [
        0x00000000_u32,
        0x00000001,
        0x0000FFFF,
        0x00010000,
        0x00011234,
        0x12345678,
        0xFFFF0000,
        0xFFFFFFFF,
    ];
    for addr in tests {
        let byte_addr = ByteAddress::from(addr);
        let (index, offset) = byte_addr.into_block_parts();
        let reconstructed = ByteAddress::from((index, offset));
        assert_eq!(
            reconstructed.value(),
            addr,
            "roundtrip failed for address {addr:#010X}",
        );
        // Also verify the parts themselves are correct.
        assert_eq!(
            index.value() as u32,
            addr >> 16,
            "wrong block index for address {addr:#010X}",
        );
        assert_eq!(
            offset.value() as u32,
            addr & 0xFFFF,
            "wrong block offset for address {addr:#010X}",
        );
    }
}

#[test]
fn block_index_first_word() {
    let indices = [0x0000, 0x0001, 0x0010, 0x00FF, 0x1000, 0xFFFF];
    for index in indices {
        let block = BlockIndex::from(index);
        let word_addr = block.base_word_address();
        let byte_addr = ByteAddress::from(word_addr);
        let (recovered_index, recovered_offset) = byte_addr.into_block_parts();
        assert_eq!(
            recovered_index.value(),
            index,
            "wrong block index for {index:#06X}",
        );
        assert_eq!(
            recovered_offset.value(),
            0,
            "non-zero offset for base word address of block {index:#06X}",
        );
    }
}

#[test]
fn write_memory_bytes_basic() {
    let mut block = [0u8; BLOCK_SIZE];
    write_memory_bytes(&mut block, &[1, 2, 3, 4], BlockOffset::from(0));
    assert_eq!(&block[0..4], &[1, 2, 3, 4]);
    // Rest of block untouched.
    assert!(block[4..].iter().all(|&b| b == 0));
}

#[test]
fn write_memory_bytes_at_offset() {
    let mut block = [0u8; BLOCK_SIZE];
    write_memory_bytes(&mut block, &[0xAB, 0xCD], BlockOffset::from(100));
    assert_eq!(block[100], 0xAB);
    assert_eq!(block[101], 0xCD);
    // Surrounding bytes untouched.
    assert!(block[..100].iter().all(|&b| b == 0));
    assert!(block[102..].iter().all(|&b| b == 0));
}

#[test]
fn write_memory_bytes_overwrites_existing() {
    let mut block = [0xFFu8; BLOCK_SIZE];
    write_memory_bytes(&mut block, &[0x00, 0x11], BlockOffset::from(10));
    assert_eq!(block[10], 0x00);
    assert_eq!(block[11], 0x11);
    // Surrounding bytes untouched.
    assert!(block[..10].iter().all(|&b| b == 0xFF));
    assert!(block[12..].iter().all(|&b| b == 0xFF));
}

#[test]
fn write_memory_bytes_at_end_of_block() {
    let mut block = [0u8; BLOCK_SIZE];
    write_memory_bytes(&mut block, &[0x12, 0x34], BlockOffset::from(u16::MAX - 1));
    assert_eq!(block[BLOCK_SIZE - 2], 0x12);
    assert_eq!(block[BLOCK_SIZE - 1], 0x34);
}

#[test]
#[should_panic]
fn write_memory_bytes_out_of_bounds() {
    let mut block = [0u8; BLOCK_SIZE];
    // Writing 2 bytes at the last offset goes one byte past the end.
    write_memory_bytes(&mut block, &[0x12, 0x34], BlockOffset::from(u16::MAX));
}

#[test]
fn block_boundary_reads_return_zero() {
    let mut block = Block::new_memory();

    // Write sentinel values at the last few offsets so we can confirm
    // they are not returned when a boundary-crossing read should fail.
    block.write_byte(BlockOffset::from(u16::MAX), 0xFF);
    block.write_byte(BlockOffset::from(u16::MAX - 1), 0xFF);
    block.write_byte(BlockOffset::from(u16::MAX - 2), 0xFF);

    // Half-word: only the last offset is too near the end.
    assert_eq!(block.read_half_word(BlockOffset::from(u16::MAX)), 0);

    // Word: the last three offsets are too near the end.
    assert_eq!(block.read_word(BlockOffset::from(u16::MAX)), 0);
    assert_eq!(block.read_word(BlockOffset::from(u16::MAX - 1)), 0);
    assert_eq!(block.read_word(BlockOffset::from(u16::MAX - 2)), 0);

    // One before the boundary should succeed and return real data.
    assert_ne!(block.read_half_word(BlockOffset::from(u16::MAX - 1)), 0);
    assert_ne!(block.read_word(BlockOffset::from(u16::MAX - 3)), 0);
}

#[test]
fn block_boundary_writes_do_not_mutate() {
    let mut block = Block::new_memory();

    // Half-word: last offset is out of bounds.
    block.write_half_word(BlockOffset::from(u16::MAX), 0xABCD);
    assert_eq!(block.read_byte(BlockOffset::from(u16::MAX)), 0);

    // Word: last three offsets are out of bounds.
    for offset in (u16::MAX - 2)..=u16::MAX {
        block.write_word(BlockOffset::from(offset), 0xDEADBEEF);
        assert_eq!(block.read_byte(BlockOffset::from(offset)), 0);
    }

    // One before the boundary should succeed.
    block.write_half_word(BlockOffset::from(u16::MAX - 1), 0xABCD);
    assert_eq!(
        block.read_half_word(BlockOffset::from(u16::MAX - 1)),
        0xABCD
    );
    block.write_word(BlockOffset::from(u16::MAX - 3), 0xDEADBEEF);
    assert_eq!(block.read_word(BlockOffset::from(u16::MAX - 3)), 0xDEADBEEF);
}

#[test]
fn block_boundary_write_returns_correct_existence() {
    // On an empty block, a boundary write should return Ignored (not Created).
    let mut empty = Block::Empty;
    assert_eq!(
        empty.write_half_word(BlockOffset::from(u16::MAX), 0xABCD),
        BlockExistence::Ignored,
    );
    assert!(
        empty.is_empty(),
        "empty block should not have been allocated"
    );

    for offset in (u16::MAX - 2)..=u16::MAX {
        let mut empty = Block::Empty;
        assert_eq!(
            empty.write_word(BlockOffset::from(offset), 0xDEADBEEF),
            BlockExistence::Ignored,
        );
        assert!(
            empty.is_empty(),
            "empty block should not have been allocated"
        );
    }

    // On an existing memory block, a boundary write should return Existed.
    let mut existing = Block::new_memory();
    assert_eq!(
        existing.write_half_word(BlockOffset::from(u16::MAX), 0xABCD),
        BlockExistence::Existed,
    );
    assert_eq!(
        existing.write_word(BlockOffset::from(u16::MAX - 2), 0xDEADBEEF),
        BlockExistence::Existed,
    );
}

#[test]
fn block_write_bytes() {
    // Basic write with no overflow.
    let mut block = Block::new_memory();
    let data = [1u8, 2, 3, 4];
    let (leftover, existence) = block.write_bytes(BlockOffset::from(0), &data);
    assert_eq!(existence, BlockExistence::Existed);
    assert!(leftover.is_empty());
    assert_eq!(block.read_byte(BlockOffset::from(0)), 1);
    assert_eq!(block.read_byte(BlockOffset::from(3)), 4);

    // Write that exactly fills to the end of the block.
    let mut block = Block::new_memory();
    let data = [0xABu8; 4];
    let (leftover, _) = block.write_bytes(BlockOffset::from(u16::MAX - 3), &data);
    assert!(leftover.is_empty());
    assert_eq!(block.read_byte(BlockOffset::from(u16::MAX)), 0xAB);

    // Write that overflows; leftover bytes are returned.
    let mut block = Block::new_memory();
    let data = [1u8, 2, 3, 4, 5, 6];
    let (leftover, existence) = block.write_bytes(BlockOffset::from(u16::MAX - 2), &data);
    assert_eq!(existence, BlockExistence::Existed);
    assert_eq!(leftover, &[4, 5, 6]);
    assert_eq!(block.read_byte(BlockOffset::from(u16::MAX - 2)), 1);
    assert_eq!(block.read_byte(BlockOffset::from(u16::MAX - 1)), 2);
    assert_eq!(block.read_byte(BlockOffset::from(u16::MAX)), 3);

    // Write starting at the very last offset; only 1 byte fits.
    let mut block = Block::new_memory();
    let data = [0xAAu8, 0xBB, 0xCC];
    let (leftover, _) = block.write_bytes(BlockOffset::from(u16::MAX), &data);
    assert_eq!(leftover, &[0xBB, 0xCC]);
    assert_eq!(block.read_byte(BlockOffset::from(u16::MAX)), 0xAA);

    // Empty slice returns immediately without allocating.
    let mut block = Block::Empty;
    let (leftover, existence) = block.write_bytes(BlockOffset::from(0), &[]);
    assert!(leftover.is_empty());
    assert_eq!(existence, BlockExistence::Ignored);
    assert!(block.is_empty());

    // All-zero write on empty block does not allocate.
    let mut block = Block::Empty;
    let (leftover, existence) = block.write_bytes(BlockOffset::from(0), &[0, 0, 0]);
    assert!(leftover.is_empty());
    assert_eq!(existence, BlockExistence::Ignored);
    assert!(block.is_empty());

    // Non-zero write on empty block allocates.
    let mut block = Block::Empty;
    let (leftover, existence) = block.write_bytes(BlockOffset::from(10), &[0, 1, 0]);
    assert!(leftover.is_empty());
    assert_eq!(existence, BlockExistence::Created);
    assert!(block.is_memory());
    assert_eq!(block.read_byte(BlockOffset::from(11)), 1);
}

use crate::is::inst;

/// Load instructions starting at word address 0 and run until `jmp 0, 0`
/// (the conventional end-of-program marker).
fn run_program(instructions: &[Instruction]) -> Vm {
    let mut vm = Vm::new();
    let len = instructions.len() as u32;
    for (i, instruction) in instructions.iter().enumerate() {
        vm.write_instruction(WordAddress::from(i as u32), instruction);
    }
    loop {
        let result = vm.execute_and_advance();
        if let Ok((instruction, _)) = &result {
            if instruction.operation() == Operation::JMP
                && instruction
                    .i_type_payload()
                    .is_some_and(|p| p.immediate_value() == 0)
            {
                break;
            }
        }
        assert!(
            vm.program_counter().value() < len,
            "program counter escaped loaded program at address {}",
            vm.program_counter().value(),
        );
    }
    vm
}

/// Assert the values of a set of (register_index, expected_value) pairs.
fn assert_registers(vm: &Vm, expected: &[(usize, u32)]) {
    for &(index, value) in expected {
        assert_eq!(
            vm.register(index),
            value,
            "register r{index} expected {value:#010X}, got {:#010X}",
            vm.register(index),
        );
    }
}

#[test]
fn vm_new() {
    let vm = Vm::new();
    assert_eq!(vm.program_counter().value(), 0);
    assert!(vm.registers().iter().all(|&r| r == 0));
    assert!(vm.blocks().iter().all(|b| b.is_empty()));
}

#[test]
fn vm_register_zero() {
    let mut vm = Vm::new();
    vm.set_register(0, 0xDEADBEEF);
    assert_eq!(vm.register(0), 0);
}

#[test]
fn vm_register_zero_prgm() {
    let vm = run_program(&[
        inst::addi(0, 0, 0xFFFF), // addi r0, r0, 0xFFFF
        inst::jmp(0, 0),
    ]);
    assert_eq!(vm.register(0), 0);
}

#[test]
fn vm_restart() {
    let mut vm = Vm::new();
    vm.set_register(1, 0xABCD);
    vm.write_byte(ByteAddress::from(0x1000), 0xFF);
    vm.set_program_counter(WordAddress::from(10));
    vm.restart();
    assert_eq!(vm.program_counter().value(), 0);
    assert!(vm.registers().iter().all(|&r| r == 0));

    // Memory is not cleared.
    assert_eq!(vm.read_byte(ByteAddress::from(0x1000)), 0xFF);
}

#[test]
fn vm_reset() {
    let mut vm = Vm::new();
    vm.set_register(1, 0xABCD);
    vm.write_byte(ByteAddress::from(0x1000), 0xFF);
    vm.set_program_counter(WordAddress::from(10));
    vm.reset();
    assert_eq!(vm.program_counter().value(), 0);
    assert!(vm.registers().iter().all(|&r| r == 0));
    assert_eq!(vm.read_byte(ByteAddress::from(0x1000)), 0);
    assert!(vm.blocks().iter().all(|b| b.is_empty()));
}

#[test]
fn vm_memory_read_write_byte() {
    let mut vm = Vm::new();
    vm.write_byte(ByteAddress::from(0x1000), 0xAB);
    assert_eq!(vm.read_byte(ByteAddress::from(0x1000)), 0xAB);

    // Surrounding bytes untouched.
    assert_eq!(vm.read_byte(ByteAddress::from(0x0FFF)), 0);
    assert_eq!(vm.read_byte(ByteAddress::from(0x1001)), 0);
}

#[test]
fn vm_memory_read_write_half_word() {
    let mut vm = Vm::new();
    vm.write_half_word(ByteAddress::from(0x2000), 0xABCD);
    assert_eq!(vm.read_half_word(ByteAddress::from(0x2000)), 0xABCD);
}

#[test]
fn vm_memory_read_write_word() {
    let mut vm = Vm::new();
    vm.write_word(ByteAddress::from(0x3000), 0xDEADBEEF);
    assert_eq!(vm.read_word(ByteAddress::from(0x3000)), 0xDEADBEEF);
}

#[test]
fn vm_memory_unwritten_reads_zero() {
    let vm = Vm::new();
    assert_eq!(vm.read_byte(ByteAddress::from(0x5000)), 0);
    assert_eq!(vm.read_half_word(ByteAddress::from(0x5000)), 0);
    assert_eq!(vm.read_word(ByteAddress::from(0x5000)), 0);
}

#[test]
fn vm_memory_zero_write_no_allocate() {
    let mut vm = Vm::new();
    vm.write_byte(ByteAddress::from(0x1000), 0);
    let (index, _) = ByteAddress::from(0x1000_u32).into_block_parts();
    assert!(vm.block(index).is_empty());
}

#[test]
fn vm_memory_boundary_read() {
    let mut vm = Vm::new();
    // Write sentinel values at the last bytes of block 0.
    let last_byte_of_block = ByteAddress::from(0x0000FFFF);
    vm.write_byte(last_byte_of_block, 0xFF);
    // A half-word or word read spanning into the next block should return 0.
    assert_eq!(vm.read_half_word(last_byte_of_block), 0);
    assert_eq!(vm.read_word(last_byte_of_block), 0);
}

#[test]
fn vm_memory_boundary_write() {
    let mut vm = Vm::new();
    let last_byte_of_block = ByteAddress::from(0x0000FFFF);
    vm.write_half_word(last_byte_of_block, 0xABCD);
    assert_eq!(vm.read_byte(last_byte_of_block), 0);
    vm.write_word(last_byte_of_block, 0xDEADBEEF);
    assert_eq!(vm.read_byte(last_byte_of_block), 0);
}

#[test]
fn inst_add() {
    let vm = run_program(&[
        inst::addi(1, 0, 10), // r1 = 10
        inst::addi(2, 0, 20), // r2 = 20
        inst::add(3, 1, 2),   // r3 = r1 + r2
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(1, 10), (2, 20), (3, 30)]);
}

#[test]
fn inst_add_wraps() {
    let vm = run_program(&[
        inst::addi(1, 0, 0xFFFF),
        inst::orui(1, 1, 0xFFFF), // r1 = 0xFFFFFFFF
        inst::addi(2, 0, 1),      // r2 = 1
        inst::add(3, 1, 2),       // r3 = wrapping 0xFFFFFFFF + 1 = 0
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0)]);
}

#[test]
fn inst_sub() {
    let vm = run_program(&[
        inst::addi(1, 0, 30),
        inst::addi(2, 0, 10),
        inst::sub(3, 1, 2), // r3 = 30 - 10
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 20)]);
}

#[test]
fn inst_sub_wraps() {
    let vm = run_program(&[
        inst::addi(1, 0, 0),
        inst::addi(2, 0, 1),
        inst::sub(3, 1, 2), // r3 = 0 - 1 = 0xFFFFFFFF
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0xFFFFFFFF)]);
}

#[test]
fn inst_shl_shr() {
    let vm = run_program(&[
        inst::addi(1, 0, 1),
        inst::addi(2, 0, 4),
        inst::shl(3, 1, 2), // r3 = 1 << 4 = 16
        inst::shr(4, 3, 2), // r4 = 16 >> 4 = 1
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 16), (4, 1)]);
}

#[test]
fn inst_sar_preserves_sign() {
    let vm = run_program(&[
        inst::addi(1, 0, 0xFFFF),
        inst::orui(1, 1, 0xFFFF), // r1 = 0xFFFFFFFF (-1 signed)
        inst::addi(2, 0, 4),
        inst::sar(3, 1, 2), // r3 = -1 >> 4 = -1 (arithmetic)
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0xFFFFFFFF)]);
}

#[test]
fn inst_and_or_xor() {
    let vm = run_program(&[
        inst::addi(1, 0, 0x00FF),
        inst::addi(2, 0, 0x0F0F),
        inst::and(3, 1, 2), // r3 = 0x000F
        inst::or(4, 1, 2),  // r4 = 0x0FFF
        inst::xor(5, 1, 2), // r5 = 0x0FF0
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0x000F), (4, 0x0FFF), (5, 0x0FF0)]);
}

#[test]
fn inst_slt_sltu() {
    let vm = run_program(&[
        inst::addi(1, 0, 0xFFFF),
        inst::orui(1, 1, 0xFFFF), // r1 = 0xFFFFFFFF (-1 signed, large unsigned)
        inst::addi(2, 0, 1),      // r2 = 1
        inst::slt(3, 1, 2),       // r3 = (-1 < 1) signed = 1
        inst::slt(4, 2, 1),       // r4 = (1 < -1) signed = 0
        inst::sltu(5, 1, 2),      // r5 = (0xFFFFFFFF < 1) unsigned = 0
        inst::sltu(6, 2, 1),      // r6 = (1 < 0xFFFFFFFF) unsigned = 1
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 1), (4, 0), (5, 0), (6, 1)]);
}

#[test]
fn inst_addi_subi() {
    let vm = run_program(&[
        inst::addi(1, 0, 100), // r1 = 100
        inst::subi(2, 1, 40),  // r2 = 60
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(1, 100), (2, 60)]);
}

#[test]
fn inst_shli_shri_sari() {
    let vm = run_program(&[
        inst::addi(1, 0, 1),
        inst::shli(2, 1, 8), // r2 = 256
        inst::shri(3, 2, 4), // r3 = 16
        inst::addi(4, 0, 0xFFFF),
        inst::orui(4, 4, 0xFFFF), // r4 = 0xFFFFFFFF
        inst::sari(5, 4, 1),      // r5 = 0xFFFFFFFF (arithmetic, sign preserved)
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(2, 256), (3, 16), (5, 0xFFFFFFFF)]);
}

#[test]
fn inst_andi_ori_xori() {
    let vm = run_program(&[
        inst::addi(1, 0, 0x00FF),
        inst::andi(2, 1, 0x0F0F), // r2 = 0x000F (lower 16 bits)
        inst::ori(3, 1, 0x0F00),  // r3 = 0x0FFF
        inst::xori(4, 1, 0x00FF), // r4 = 0x0000
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(2, 0x000F), (3, 0x0FFF), (4, 0x0000)]);
}

#[test]
fn inst_andui_orui_xorui() {
    // These operate on the upper 16 bits.
    let vm = run_program(&[
        inst::addi(1, 0, 0xFFFF),
        inst::orui(1, 1, 0xFFFF),  // r1 = 0xFFFFFFFF
        inst::andui(2, 1, 0x0F0F), // r2 = 0x0F0FFFFF
        inst::orui(3, 0, 0xABCD),  // r3 = 0xABCD0000
        inst::xorui(4, 1, 0xFFFF), // r4 = 0x0000FFFF
        inst::jmp(0, 0),
    ]);
    assert_registers(
        &vm,
        &[
            (1, 0xFFFFFFFF),
            (2, 0x0F0FFFFF),
            (3, 0xABCD0000),
            (4, 0x0000FFFF),
        ],
    );
}

#[test]
fn inst_slti_sltui() {
    let vm = run_program(&[
        inst::addi(1, 0, 0xFFFF),
        inst::orui(1, 1, 0xFFFF),     // r1 = 0xFFFFFFFF (-1 signed)
        inst::slti(2, 1, 0),          // r2 = (-1 < 0) signed = 1
        inst::slti(3, 0, 0xFFFF_u16), // r3 = (0 < -1 as i16) signed = 0
        inst::sltui(4, 1, 1),         // r4 = (0xFFFFFFFF < 1) unsigned = 0
        inst::sltui(5, 0, 1),         // r5 = (0 < 1) unsigned = 1
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(2, 1), (3, 0), (4, 0), (5, 1)]);
}

#[test]
fn inst_stw_ldw() {
    let vm = run_program(&[
        inst::addi(1, 0, 0x00FF),
        inst::orui(1, 1, 0x00FF), // r1 = 0x00FF00FF
        inst::addi(2, 0, 0x0200), // r2 = base address 0x0200 (byte addr 0x0800)
        inst::shli(2, 2, 2),      // r2 = 0x0800 (byte address)
        inst::stw(1, 2, 0),       // mem[r2 + 0] = r1
        inst::ldw(3, 2, 0),       // r3 = mem[r2 + 0]
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0x00FF00FF)]);
}

#[test]
fn inst_sthw_ldhw_ldhwu() {
    let vm = run_program(&[
        inst::addi(1, 0, 0x8000_u16), // r1 = 0x8000 (sign bit set)
        inst::addi(2, 0, 0x0200),
        inst::shli(2, 2, 2), // r2 = byte address 0x0800
        inst::sthw(1, 2, 0),
        inst::ldhw(3, 2, 0),  // r3 = sign-extended 0x8000 = 0xFFFF8000
        inst::ldhwu(4, 2, 0), // r4 = zero-extended 0x8000 = 0x00008000
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0xFFFF8000), (4, 0x00008000)]);
}

#[test]
fn inst_stb_ldb_ldbu() {
    let vm = run_program(&[
        inst::addi(1, 0, 0x0080_u16), // r1 = 0x80 (sign bit set as byte)
        inst::addi(2, 0, 0x0200),
        inst::shli(2, 2, 2), // r2 = byte address 0x0800
        inst::stb(1, 2, 0),
        inst::ldb(3, 2, 0),  // r3 = sign-extended 0x80 = 0xFFFFFF80
        inst::ldbu(4, 2, 0), // r4 = zero-extended 0x80 = 0x00000080
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0xFFFFFF80), (4, 0x00000080)]);
}

#[test]
fn inst_load_store_with_offset() {
    let vm = run_program(&[
        inst::addi(1, 0, 0x1234),
        inst::addi(2, 0, 0x0200),
        inst::shli(2, 2, 2), // r2 = byte base address 0x0800
        inst::stw(1, 2, 4),  // mem[r2 + 4] = r1
        inst::stw(1, 2, 8),  // mem[r2 + 8] = r1
        inst::ldw(3, 2, 4),  // r3 = mem[r2 + 4]
        inst::ldw(4, 2, 8),  // r4 = mem[r2 + 8]
        inst::ldw(5, 2, 0),  // r5 = mem[r2 + 0] = 0 (unwritten)
        inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0x1234), (4, 0x1234), (5, 0)]);
}

#[test]
fn inst_jmp_links_return_address() {
    // jmp stores pc+1 in the link register before jumping.
    let vm = run_program(&[
        /* 0 */ inst::jmp(1, 2), // r1 = 1, jump to pc+2 = addr 2
        /* 1 */ inst::addi(2, 0, 0xDEAD), // skipped
        /* 2 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(1, 1), (2, 0)]);
}

#[test]
fn inst_jmpr_jumps_relative_to_register() {
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 3), // r1 = 3 (target word addr)
        /* 1 */ inst::jmpr(2, 1, -1), // r2 = 2, jump to r1 + (-1) = addr 2
        /* 2 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(2, 2)]);
}

#[test]
fn inst_beq_taken() {
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 5),
        /* 1 */ inst::addi(2, 0, 5),
        /* 2 */ inst::beq(1, 2, 1), // r1 == r2, jump to pc+1 = addr 3
        /* 3 */ inst::addi(3, 0, 0xABCD), // reached
        /* 4 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0xABCD)]);
}

#[test]
fn inst_beq_not_taken() {
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 5),
        /* 1 */ inst::addi(2, 0, 6),
        /* 2 */ inst::beq(1, 2, 1), // r1 != r2, not taken
        /* 3 */ inst::addi(3, 0, 0xABCD), // still reached (fell through)
        /* 4 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0xABCD)]);
}

#[test]
fn inst_bne_taken() {
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 5),
        /* 1 */ inst::addi(2, 0, 6),
        /* 2 */ inst::bne(1, 2, 2), // r1 != r2, jump to pc+2 = addr 4
        /* 3 */ inst::addi(3, 0, 0xDEAD), // skipped
        /* 4 */ inst::addi(4, 0, 0xBEEF), // reached
        /* 5 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0), (4, 0xBEEF)]);
}

#[test]
fn inst_bne_not_taken() {
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 5),
        /* 1 */ inst::addi(2, 0, 5),
        /* 2 */ inst::bne(1, 2, 2), // r1 == r2, not taken
        /* 3 */ inst::addi(3, 0, 0xABCD), // reached
        /* 4 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(3, 0xABCD)]);
}

#[test]
fn pc_wraps_at_max() {
    // Set the PC to the last valid word address and advance; should wrap to 0.
    let mut vm = Vm::new();
    vm.set_program_counter(WordAddress::from(0x3FFFFFFF));
    vm.advance();
    assert_eq!(vm.program_counter().value(), 0);
}

#[test]
fn run_until_loop() {
    let vm = run_program(&[
        inst::addi(1, 0, 42),
        inst::jmp(0, 0),
        inst::addi(2, 0, 99), // never reached
    ]);
    assert_registers(&vm, &[(1, 42), (2, 0)]);
}

#[test]
fn run_to_pc() {
    let mut vm = Vm::new();
    let instructions = [
        inst::addi(1, 0, 1), // addr 0
        inst::addi(2, 0, 2), // addr 1
        inst::addi(3, 0, 3), // addr 2; stop here, don't execute
    ];
    for (i, inst) in instructions.iter().enumerate() {
        vm.write_instruction(WordAddress::from(i as u32), inst);
    }
    vm.run_to_pc(WordAddress::from(2));
    assert_registers(&vm, &[(1, 1), (2, 2), (3, 0)]);
}

#[test]
fn run_until_jumped_stops_on_first_jump() {
    let mut vm = Vm::new();
    let instructions = [
        inst::addi(1, 0, 1), // addr 0
        inst::jmp(2, 1),     // addr 1; jumps to addr 2, stops here
        inst::addi(3, 0, 3), // addr 2; not executed yet
        inst::jmp(0, 0),
    ];
    for (i, inst) in instructions.iter().enumerate() {
        vm.write_instruction(WordAddress::from(i as u32), inst);
    }
    vm.run_until_jumped();
    assert_registers(&vm, &[(1, 1), (2, 2), (3, 0)]);
    assert_eq!(vm.program_counter().value(), 2);
}

#[test]
fn run_while_valid_stops_on_invalid_instruction() {
    let mut vm = Vm::new();
    vm.write_instruction(WordAddress::from(0), &inst::addi(1, 0, 1));
    // Word address 1 is left as 0x00000000, which is a valid noop.
    // Write a known-invalid opcode at word address 2.
    let invalid_word = 0x07 << 26_u32; // opcode 0x07 is invalid
    vm.write_word(ByteAddress::from(8), invalid_word); // word addr 2 = byte addr 8
    vm.run_while_valid();
    // r1 should have been set, and the PC should be past the invalid instruction.
    assert_eq!(vm.register(1), 1);
    assert_eq!(vm.program_counter().value(), 3);
}

#[test]
fn program_sum_1_to_10() {
    // Compute sum of 1..=10 in r1 using a counted loop.
    // r1 = accumulator, r2 = counter, r3 = limit (10)
    // Loop: r1 += r2; r2 += 1; if r2 != r3+1 branch back
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 0), // r1 = 0 (sum)
        /* 1 */ inst::addi(2, 0, 1), // r2 = 1 (counter)
        /* 2 */ inst::addi(3, 0, 11), // r3 = 11 (loop exit value)
        /* 3 */ inst::add(1, 1, 2), // r1 += r2
        /* 4 */ inst::addi(2, 2, 1), // r2 += 1
        /* 5 */
        inst::bne(2, 3, -3), // if r2 != 11, branch back to addr 3 (offset -3 from pc 6 = 3)
        /* 6 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(1, 55)]);
}

#[test]
fn program_memory_copy() {
    // Copy a word from one address to another via load/store.
    // Source: byte addr 0x1000, dest: byte addr 0x2000.
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 0x1000_u16), // r1 = source byte addr
        /* 1 */ inst::addi(2, 0, 0x2000_u16), // r2 = dest byte addr (won't fit in addi)
        /* 2 */ inst::ldw(3, 1, 0), // r3 = mem[0x1000]; will be 0 since unwritten
        /* 3 */ inst::stw(3, 2, 0), // mem[0x2000] = r3
        /* 4 */ inst::jmp(0, 0),
    ]);
    // Since the source is uninitialized, both should read 0.
    assert_eq!(vm.read_word(ByteAddress::from(0x2000)), 0);
}

#[test]
fn program_fibonacci() {
    // Compute fib(8) = 21 iteratively.
    // r1 = a (fib(n-2)), r2 = b (fib(n-1)), r3 = counter, r4 = limit, r5 = temp
    let vm = run_program(&[
        /* 0 */ inst::addi(1, 0, 0), // r1 = 0
        /* 1 */ inst::addi(2, 0, 1), // r2 = 1
        /* 2 */ inst::addi(3, 0, 0), // r3 = 0 (counter)
        /* 3 */ inst::addi(4, 0, 7), // r4 = 7 (iterations for fib(8))
        /* 4 */ inst::add(5, 1, 2), // r5 = r1 + r2
        /* 5 */ inst::addi(1, 2, 0), // r1 = r2  (addi r1, r2, 0)
        /* 6 */ inst::addi(2, 5, 0), // r2 = r5
        /* 7 */ inst::addi(3, 3, 1), // r3++
        /* 8 */ inst::bne(3, 4, -5), // if r3 != 7, loop back to addr 4
        /* 9 */ inst::jmp(0, 0),
    ]);
    assert_registers(&vm, &[(2, 21)]);
}
