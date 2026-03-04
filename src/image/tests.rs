use crate::vm::{IoController, WordAddress};

use super::*;

/// Serialize an ImageEntryRef to bytes using write_to.
fn entry_to_bytes(entry: &ImageEntryRef) -> Vec<u8> {
    let mut buf = Vec::new();
    entry.write_to(&mut buf).unwrap();
    buf
}

#[test]
fn image_entry_new_valid() {
    let entry = ImageEntry::new(ByteAddress::from(0x1000), vec![1, 2, 3, 4]).unwrap();
    assert_eq!(entry.address().value(), 0x1000);
    assert_eq!(entry.data(), &[1, 2, 3, 4]);
}

#[test]
fn image_entry_new_empty() {
    let result = ImageEntry::new(ByteAddress::from(0), vec![]);
    assert!(matches!(result, Err(ImageEntryError::Empty)));
}

#[test]
fn image_entry_single_byte_end() {
    // A single byte at the very last address should be valid.
    let result = ImageEntry::new(ByteAddress::from(u32::MAX), vec![0xFF]);
    assert!(result.is_ok());
}

#[test]
fn image_entry_new_overflow() {
    // Two bytes at the last address would extend past the end.
    let result = ImageEntry::new(ByteAddress::from(u32::MAX), vec![1, 2]);
    assert!(matches!(result, Err(ImageEntryError::Overflow)));

    // A large write near the end that wraps.
    let result = ImageEntry::new(ByteAddress::from(u32::MAX - 3), vec![1, 2, 3, 4, 5]);
    assert!(matches!(result, Err(ImageEntryError::Overflow)));
}

#[test]
fn image_entry_write_to_format() {
    // Format is: 4-byte BE start, 4-byte BE end, then data.
    let entry = ImageEntry::new(ByteAddress::from(0x00001000), vec![0xAA, 0xBB, 0xCC]).unwrap();
    let ref_entry = ImageEntryRef::from(entry);
    let bytes = entry_to_bytes(&ref_entry);

    // Start address: 0x00001000
    assert_eq!(&bytes[0..4], &0x00001000_u32.to_be_bytes());
    // End address: 0x00001002 (start + len - 1)
    assert_eq!(&bytes[4..8], &0x00001002_u32.to_be_bytes());
    // Data
    assert_eq!(&bytes[8..], &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn image_entry_write_to_single_byte() {
    let entry = ImageEntry::new(ByteAddress::from(0x0000ABCD), vec![0x42]).unwrap();
    let ref_entry = ImageEntryRef::from(entry);
    let bytes = entry_to_bytes(&ref_entry);

    assert_eq!(&bytes[0..4], &0x0000ABCD_u32.to_be_bytes());
    // End == start for a single byte.
    assert_eq!(&bytes[4..8], &0x0000ABCD_u32.to_be_bytes());
    assert_eq!(&bytes[8..], &[0x42]);
}

#[test]
fn image_entry_write_to_at_address_zero() {
    let entry = ImageEntry::new(ByteAddress::from(0), vec![1, 2, 3]).unwrap();
    let ref_entry = ImageEntryRef::from(entry);
    let bytes = entry_to_bytes(&ref_entry);

    assert_eq!(&bytes[0..4], &0_u32.to_be_bytes());
    assert_eq!(&bytes[4..8], &2_u32.to_be_bytes());
    assert_eq!(&bytes[8..], &[1, 2, 3]);
}

#[test]
fn image_entry_serialization_round_trip() {
    let original = ImageEntry::new(ByteAddress::from(0x5000), vec![10, 20, 30, 40]).unwrap();
    let ref_entry = ImageEntryRef::from(&original);
    let bytes = entry_to_bytes(&ref_entry);

    // Parse back via from_byte_iter.
    let mut iter = bytes.into_iter();
    let recovered = ImageEntry::from_byte_iter(&mut iter).unwrap();

    assert_eq!(recovered.address().value(), original.address().value());
    assert_eq!(recovered.data(), original.data());
    // Iterator should be exhausted.
    assert!(iter.next().is_none());
}

#[test]
fn image_entry_from_byte_iter_incomplete() {
    // Too few bytes to read start/end offsets.
    let mut iter = [0x00, 0x00, 0x10].into_iter();
    let result = ImageEntry::from_byte_iter(&mut iter);
    assert!(matches!(result, Err(ImageEntryError::Incomplete)));

    // Exactly 8 bytes (start + end) but no data; data length is 1
    // so the data vec will just be empty (truncated), not an error.
    // Empty iterator from the start.
    let mut iter = [].into_iter();
    let result = ImageEntry::from_byte_iter(&mut iter);
    assert!(matches!(result, Err(ImageEntryError::Incomplete)));
}

#[test]
fn image_entry_from_byte_iter_bad_offsets() {
    // end < start should produce BadOffsets.
    let start: u32 = 0x00002000;
    let end: u32 = 0x00001000; // end before start
    let mut bytes = Vec::new();
    bytes.extend(start.to_be_bytes());
    bytes.extend(end.to_be_bytes());
    let mut iter = bytes.into_iter();
    let result = ImageEntry::from_byte_iter(&mut iter);
    assert!(matches!(result, Err(ImageEntryError::BadOffsets { .. })));
}

#[test]
fn image_entry_from_byte_iter_multiple_sequential() {
    // Pack two entries back to back and parse them both.
    let e1 = ImageEntry::new(ByteAddress::from(0x0100), vec![1, 2]).unwrap();
    let e2 = ImageEntry::new(ByteAddress::from(0x0200), vec![3, 4, 5]).unwrap();
    let mut bytes = Vec::new();
    entry_to_bytes(&ImageEntryRef::from(&e1))
        .into_iter()
        .for_each(|b| bytes.push(b));
    entry_to_bytes(&ImageEntryRef::from(&e2))
        .into_iter()
        .for_each(|b| bytes.push(b));

    let mut iter = bytes.into_iter();
    let r1 = ImageEntry::from_byte_iter(&mut iter).unwrap();
    let r2 = ImageEntry::from_byte_iter(&mut iter).unwrap();

    assert_eq!(r1.address().value(), 0x0100);
    assert_eq!(r1.data(), &[1, 2]);
    assert_eq!(r2.address().value(), 0x0200);
    assert_eq!(r2.data(), &[3, 4, 5]);
}

#[test]
fn image_new_is_empty() {
    let image = Image::new();
    assert!(image.entries().next().is_none());
}

#[test]
fn image_add_and_entries() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3]).unwrap());
    image.add(ImageEntry::new(ByteAddress::from(0x200), vec![4, 5, 6]).unwrap());

    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address().value(), 0x100);
    assert_eq!(entries[0].data(), &[1, 2, 3]);
    assert_eq!(entries[1].address().value(), 0x200);
    assert_eq!(entries[1].data(), &[4, 5, 6]);
}

#[test]
fn image_clear_fully_covered_entry() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3, 4]).unwrap());
    image.clear(0x100, 0x103);
    assert!(image.entries().next().is_none());
}

#[test]
fn image_clear_misses_entry() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3, 4]).unwrap());
    // Clear a range that doesn't overlap at all.
    image.clear(0x200, 0x300);
    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].data(), &[1, 2, 3, 4]);
}

#[test]
fn image_clear_partial_overlap_start() {
    let mut image = Image::new();
    // Entry at 0x100..=0x103.
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3, 4]).unwrap());
    // Clear 0x100..=0x101, leaving 0x102..=0x103.
    image.clear(0x100, 0x101);
    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0x102);
    assert_eq!(entries[0].data(), &[3, 4]);
}

#[test]
fn image_clear_partial_overlap_end() {
    let mut image = Image::new();
    // Entry at 0x100..=0x103.
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3, 4]).unwrap());
    // Clear 0x102..=0x103, leaving 0x100..=0x101.
    image.clear(0x102, 0x103);
    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0x100);
    assert_eq!(entries[0].data(), &[1, 2]);
}

#[test]
fn image_clear_spans_multiple_entries() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2]).unwrap());
    image.add(ImageEntry::new(ByteAddress::from(0x200), vec![3, 4]).unwrap());
    image.add(ImageEntry::new(ByteAddress::from(0x300), vec![5, 6]).unwrap());
    // Clear covers first two entries entirely.
    image.clear(0x100, 0x201);
    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0x300);
}

#[test]
fn image_clear_middle_of_entry() {
    let mut image = Image::new();
    // Entry at 0x100..=0x105: [1, 2, 3, 4, 5, 6]
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3, 4, 5, 6]).unwrap());
    // Clear the middle two bytes 0x102..=0x103, should leave two entries:
    // [1, 2] at 0x100 and [5, 6] at 0x104.
    image.clear(0x102, 0x103);
    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address().value(), 0x100);
    assert_eq!(entries[0].data(), &[1, 2]);
    assert_eq!(entries[1].address().value(), 0x104);
    assert_eq!(entries[1].data(), &[5, 6]);
}

#[test]
fn image_clear_exact_entry_bounds() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3, 4]).unwrap());
    // Clear range exactly matches entry start and end.
    image.clear(0x100, 0x103);
    assert!(image.entries().next().is_none());
}

#[test]
fn image_clear_entire_address_space() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2]).unwrap());
    image.add(ImageEntry::new(ByteAddress::from(0x200), vec![3, 4]).unwrap());
    image.add(ImageEntry::new(ByteAddress::from(0x300), vec![5, 6]).unwrap());
    image.clear(0, u32::MAX);
    assert!(image.entries().next().is_none());
}

#[test]
fn image_clear_between_entries() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2]).unwrap());
    image.add(ImageEntry::new(ByteAddress::from(0x200), vec![3, 4]).unwrap());
    // Clear the gap between them; neither entry should be affected.
    image.clear(0x102, 0x1FF);
    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].data(), &[1, 2]);
    assert_eq!(entries[1].data(), &[3, 4]);
}

#[test]
fn builder_cursor_zero() {
    let builder = ImageBuilder::new();
    assert_eq!(builder.cursor().value(), 0);
}

#[test]
fn builder_cursor_advances_after_writes() {
    let builder = ImageBuilder::new().write_byte(0xAA);
    assert_eq!(builder.cursor().value(), 1);

    let builder = ImageBuilder::new().write_half_word(0xAABB);
    assert_eq!(builder.cursor().value(), 2);

    let builder = ImageBuilder::new().write_word(0xAABBCCDD);
    assert_eq!(builder.cursor().value(), 4);

    let builder = ImageBuilder::new().write_bytes(vec![1, 2, 3, 4, 5]);
    assert_eq!(builder.cursor().value(), 5);

    let builder = ImageBuilder::new().write_half_words(vec![1, 2, 3]);
    assert_eq!(builder.cursor().value(), 6);

    let builder = ImageBuilder::new().write_words(vec![1, 2]);
    assert_eq!(builder.cursor().value(), 8);

    let builder = ImageBuilder::new().write_ascii("hello".to_string());
    assert_eq!(builder.cursor().value(), 5);
}

#[test]
fn builder_cursor_advances_across_chains() {
    let builder = ImageBuilder::new()
        .write_byte(1) // cursor: 0 -> 1
        .write_half_word(2) // cursor: 1 -> 3
        .write_word(3); // cursor: 3 -> 7
    assert_eq!(builder.cursor().value(), 7);
}

#[test]
fn builder_seek_moves_cursor() {
    let builder = ImageBuilder::new().seek(ByteAddress::from(0x1000));
    assert_eq!(builder.cursor().value(), 0x1000);
}

#[test]
fn builder_seek_forward() {
    // Seeking forward should not mark as non-sequential.
    let builder = ImageBuilder::new()
        .write_word(0)
        .seek(ByteAddress::from(0x100));
    assert!(builder.error().is_none());
    // A sequential write after forward seek should still merge if close enough.
    let builder = builder.write_word(1);
    assert!(builder.error().is_none());
}

#[test]
fn builder_seek_backward() {
    // After seeking backward, writes should still succeed but use the
    // non-sequential (sorted insertion) path.
    let builder = ImageBuilder::new()
        .write_word(0xAAAAAAAA) // at 0
        .seek(ByteAddress::from(0x100))
        .write_word(0xBBBBBBBB) // at 0x100
        .seek(ByteAddress::from(0x50)) // backward
        .write_word(0xCCCCCCCC); // at 0x50
    assert!(builder.error().is_none());
    let vm: Vm = builder.build().unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(0)), 0xAAAAAAAA);
    assert_eq!(vm.read_word(ByteAddress::from(0x50)), 0xCCCCCCCC);
    assert_eq!(vm.read_word(ByteAddress::from(0x100)), 0xBBBBBBBB);
}

#[test]
fn builder_advance() {
    let builder = ImageBuilder::new().advance(0x100);
    assert_eq!(builder.cursor().value(), 0x100);

    let builder = ImageBuilder::new().write_byte(1).advance(3);
    assert_eq!(builder.cursor().value(), 4);
}

#[test]
fn builder_advance_overflow() {
    let builder = ImageBuilder::new()
        .seek(ByteAddress::from(u32::MAX))
        .advance(1);
    assert!(builder.error().is_some());
}

#[test]
fn builder_write_byte() {
    let vm: Vm = ImageBuilder::new().write_byte(0xAB).build().unwrap();
    assert_eq!(vm.read_byte(ByteAddress::from(0)), 0xAB);
}

#[test]
fn builder_write_half_word() {
    let vm: Vm = ImageBuilder::new().write_half_word(0xABCD).build().unwrap();
    assert_eq!(vm.read_byte(ByteAddress::from(0)), 0xAB);
    assert_eq!(vm.read_byte(ByteAddress::from(1)), 0xCD);
    assert_eq!(vm.read_half_word(ByteAddress::from(0)), 0xABCD);
}

#[test]
fn builder_write_word() {
    let vm: Vm = ImageBuilder::new().write_word(0xDEADBEEF).build().unwrap();
    assert_eq!(vm.read_byte(ByteAddress::from(0)), 0xDE);
    assert_eq!(vm.read_byte(ByteAddress::from(1)), 0xAD);
    assert_eq!(vm.read_byte(ByteAddress::from(2)), 0xBE);
    assert_eq!(vm.read_byte(ByteAddress::from(3)), 0xEF);
    assert_eq!(vm.read_word(ByteAddress::from(0)), 0xDEADBEEF);
}

#[test]
fn builder_write_bytes() {
    let vm: Vm = ImageBuilder::new()
        .write_bytes(vec![1, 2, 3, 4, 5])
        .build()
        .unwrap();
    for i in 0..5_u32 {
        assert_eq!(vm.read_byte(ByteAddress::from(i)), i as u8 + 1);
    }
}

#[test]
fn builder_write_half_words() {
    let vm: Vm = ImageBuilder::new()
        .write_half_words(vec![0x0102, 0x0304])
        .build()
        .unwrap();
    assert_eq!(vm.read_byte(ByteAddress::from(0)), 0x01);
    assert_eq!(vm.read_byte(ByteAddress::from(1)), 0x02);
    assert_eq!(vm.read_byte(ByteAddress::from(2)), 0x03);
    assert_eq!(vm.read_byte(ByteAddress::from(3)), 0x04);
}

#[test]
fn builder_write_words() {
    let vm: Vm = ImageBuilder::new()
        .write_words(vec![0x01020304, 0x05060708])
        .build()
        .unwrap();
    for (i, expected) in [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        .iter()
        .enumerate()
    {
        assert_eq!(vm.read_byte(ByteAddress::from(i as u32)), *expected);
    }
}

#[test]
fn builder_write_ascii() {
    let vm: Vm = ImageBuilder::new()
        .write_ascii("Hi!".to_string())
        .build()
        .unwrap();
    assert_eq!(vm.read_byte(ByteAddress::from(0)), b'H');
    assert_eq!(vm.read_byte(ByteAddress::from(1)), b'i');
    assert_eq!(vm.read_byte(ByteAddress::from(2)), b'!');
}

#[test]
fn builder_write_instruction() {
    use crate::is::inst;
    let instruction = inst::addi(1, 0, 42);
    let expected_word = instruction.encode();
    let vm: Vm = ImageBuilder::new()
        .write_instruction(&instruction)
        .build()
        .unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(0)), expected_word);
}

#[test]
fn builder_write_instructions() {
    use crate::is::inst;
    let instructions = [
        inst::addi(1, 0, 1),
        inst::addi(2, 0, 2),
        inst::add(3, 1, 2),
        inst::jmp(0, 0),
    ];
    let mut vm: Vm = ImageBuilder::new()
        .write_instructions(&instructions)
        .build()
        .unwrap();
    // Verify each instruction encodes correctly at its word address.
    for (i, instruction) in instructions.iter().enumerate() {
        let byte_addr = ByteAddress::from(i as u32 * 4);
        assert_eq!(vm.read_word(byte_addr), instruction.encode());
    }
    // Also verify the program actually runs correctly.
    vm.set_program_counter(WordAddress::from(0));
    vm.run_until_loop();
    assert_eq!(vm.register(1), 1);
    assert_eq!(vm.register(2), 2);
    assert_eq!(vm.register(3), 3);
}

#[test]
fn builder_write_chains() {
    let vm: Vm = ImageBuilder::new()
        .write_byte(0x11) // byte addr 0
        .write_half_word(0x2233) // byte addr 1
        .write_word(0x44556677) // byte addr 3
        .build()
        .unwrap();
    assert_eq!(vm.read_byte(ByteAddress::from(0)), 0x11);
    assert_eq!(vm.read_half_word(ByteAddress::from(1)), 0x2233);
    assert_eq!(vm.read_word(ByteAddress::from(3)), 0x44556677);
}

#[test]
fn builder_write_instruction_aligned() {
    use crate::is::inst;
    let instruction = inst::addi(1, 0, 42);
    // Cursor starts at 0, which is already word-aligned.
    let builder = ImageBuilder::new().write_instruction(&instruction);
    assert_eq!(builder.cursor().value(), 4);
    let vm: Vm = builder.build().unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(0)), instruction.encode());
}

#[test]
fn builder_write_instruction_offset_1() {
    use crate::is::inst;
    let instruction = inst::addi(1, 0, 42);
    // Cursor at 1 should round up to 4.
    let builder = ImageBuilder::new()
        .write_byte(0xFF)
        .write_instruction(&instruction);
    assert_eq!(builder.cursor().value(), 8);
    let vm: Vm = builder.build().unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(4)), instruction.encode());
}

#[test]
fn builder_write_instruction_offset_2() {
    use crate::is::inst;
    let instruction = inst::addi(1, 0, 42);
    // Cursor at 2 should round up to 4.
    let builder = ImageBuilder::new()
        .write_half_word(0xABCD)
        .write_instruction(&instruction);
    assert_eq!(builder.cursor().value(), 8);
    let vm: Vm = builder.build().unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(4)), instruction.encode());
}

#[test]
fn builder_write_instruction_offset_3() {
    use crate::is::inst;
    let instruction = inst::addi(1, 0, 42);
    // Cursor at 3 should round up to 4.
    let builder = ImageBuilder::new()
        .write_byte(0x11)
        .write_half_word(0x2233)
        .write_instruction(&instruction);
    assert_eq!(builder.cursor().value(), 8);
    let vm: Vm = builder.build().unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(4)), instruction.encode());
}

#[test]
fn builder_write_instructions_aligns() {
    use crate::is::inst;
    let instructions = [inst::addi(1, 0, 1), inst::jmp(0, 0)];
    // Cursor at 1 should round up to 4 before writing instructions.
    let builder = ImageBuilder::new()
        .write_byte(0xFF)
        .write_instructions(&instructions);
    assert_eq!(builder.cursor().value(), 12);
    let vm: Vm = builder.build().unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(4)), instructions[0].encode());
    assert_eq!(vm.read_word(ByteAddress::from(8)), instructions[1].encode());
}

#[test]
fn builder_write_instruction_padded() {
    use crate::is::inst;
    let instruction = inst::addi(1, 0, 42);
    // Write a byte at 0, then an instruction; gap bytes 1,2,3 should be 0.
    let vm: Vm = ImageBuilder::new()
        .write_byte(0xFF)
        .write_instruction(&instruction)
        .build()
        .unwrap();
    assert_eq!(vm.read_byte(ByteAddress::from(1)), 0);
    assert_eq!(vm.read_byte(ByteAddress::from(2)), 0);
    assert_eq!(vm.read_byte(ByteAddress::from(3)), 0);
}

#[test]
fn builder_merge_adjacent_writes() {
    // Two writes with no gap should merge into one entry.
    let entries: Image = ImageBuilder::new()
        .write_word(0x11223344)
        .write_word(0x55667788)
        .build()
        .unwrap();
    let entries: Vec<_> = entries.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0);
    assert_eq!(
        entries[0].data(),
        &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
    );
}

#[test]
fn builder_merge_writes_within_buffer() {
    // A gap of 7 bytes (just within the 8-byte threshold) should merge.
    let entries: Image = ImageBuilder::new()
        .write_byte(0xAA)
        .advance(7)
        .write_byte(0xBB)
        .build()
        .unwrap();
    let entries: Vec<_> = entries.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0);
    // 7 zero-padding bytes between 0xAA and 0xBB.
    assert_eq!(entries[0].data(), &[0xAA, 0, 0, 0, 0, 0, 0, 0, 0xBB]);
}

#[test]
fn builder_no_merge_at_buffer_boundary() {
    // A gap of exactly 8 bytes should NOT merge.
    let entries: Image = ImageBuilder::new()
        .write_byte(0xAA)
        .advance(8)
        .write_byte(0xBB)
        .build()
        .unwrap();
    let entries: Vec<_> = entries.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address().value(), 0);
    assert_eq!(entries[0].data(), &[0xAA]);
    assert_eq!(entries[1].address().value(), 9);
    assert_eq!(entries[1].data(), &[0xBB]);
}

#[test]
fn builder_no_merge_beyond_buffer() {
    // A gap larger than 8 bytes should produce separate entries.
    let entries: Image = ImageBuilder::new()
        .write_word(0x11223344)
        .advance(100)
        .write_word(0x55667788)
        .build()
        .unwrap();
    let entries: Vec<_> = entries.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address().value(), 0);
    assert_eq!(entries[1].address().value(), 104);
}

#[test]
fn builder_merge_three_writes() {
    // Three writes each within 8 bytes of each other should all merge.
    let entries: Image = ImageBuilder::new()
        .write_byte(0xAA)
        .advance(3)
        .write_byte(0xBB)
        .advance(3)
        .write_byte(0xCC)
        .build()
        .unwrap();
    let entries: Vec<_> = entries.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].data(), &[0xAA, 0, 0, 0, 0xBB, 0, 0, 0, 0xCC]);
}

#[test]
fn builder_merge_nonsequential_within_buffer() {
    let entries: Image = ImageBuilder::new()
        .write_byte(0xAA) // addr 0
        .seek(ByteAddress::from(0x08))
        .write_byte(0xCC) // addr 0x08
        .seek(ByteAddress::from(0x03)) // backward seek
        .write_byte(0xBB) // addr 0x03, gap of 2 from 0xAA, gap of 4 to 0xCC
        .build()
        .unwrap();
    let entries: Vec<_> = entries.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0);
    assert_eq!(entries[0].data(), &[0xAA, 0, 0, 0xBB, 0, 0, 0, 0, 0xCC]);
}

#[test]
fn builder_error_overflow_write_past_end() {
    let builder = ImageBuilder::new()
        .seek(ByteAddress::from(u32::MAX))
        .write_half_word(0xABCD);
    assert!(matches!(
        builder.error(),
        Some(ImageBuildError::Overflow(..))
    ));
}

#[test]
fn builder_error_overlap_same_address() {
    let builder = ImageBuilder::new()
        .write_word(0x11223344)
        .seek(ByteAddress::from(0))
        .write_word(0x55667788);
    assert!(matches!(
        builder.error(),
        Some(ImageBuildError::Overlap { .. })
    ));
}

#[test]
fn builder_error_overlap_partial() {
    // Write 4 bytes at 0, then 4 bytes at 2; overlaps by 2 bytes.
    let builder = ImageBuilder::new()
        .write_word(0x11223344)
        .seek(ByteAddress::from(2))
        .write_word(0x55667788);
    assert!(matches!(
        builder.error(),
        Some(ImageBuildError::Overlap { .. })
    ));
}

#[test]
fn builder_error_overlap_nonsequential() {
    // Write out of order such that the inserted write overlaps an existing one.
    let builder = ImageBuilder::new()
        .seek(ByteAddress::from(0x10))
        .write_word(0xAAAAAAAA)
        .seek(ByteAddress::from(0x00))
        .write_words(vec![0; 8]); // 32 bytes, reaches past 0x10
    assert!(matches!(
        builder.error(),
        Some(ImageBuildError::Overlap { .. })
    ));
}

#[test]
fn builder_error_is_sticky() {
    // After an error, further writes are silently ignored.
    let builder = ImageBuilder::new()
        .write_word(0x11223344)
        .seek(ByteAddress::from(0))
        .write_word(0x55667788) // causes overlap error
        .seek(ByteAddress::from(0x1000))
        .write_word(0xDEADBEEF); // should be ignored
    assert!(builder.error().is_some());
    let result: Result<Vm, _> = builder.build();
    assert!(result.is_err());
}

#[test]
fn builder_error_build_returns_err() {
    let result: Result<Vm, _> = ImageBuilder::new()
        .seek(ByteAddress::from(u32::MAX))
        .write_half_word(0xABCD)
        .build();
    assert!(result.is_err());
}

#[test]
fn builder_out_of_order_entries_are_sorted() {
    let entries: Image = ImageBuilder::new()
        .seek(ByteAddress::from(0x300))
        .write_byte(0xCC)
        .seek(ByteAddress::from(0x100))
        .write_byte(0xAA)
        .seek(ByteAddress::from(0x200))
        .write_byte(0xBB)
        .build()
        .unwrap();
    let entries: Vec<_> = entries.entries().collect();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].address().value(), 0x100);
    assert_eq!(entries[0].data(), &[0xAA]);
    assert_eq!(entries[1].address().value(), 0x200);
    assert_eq!(entries[1].data(), &[0xBB]);
    assert_eq!(entries[2].address().value(), 0x300);
    assert_eq!(entries[2].data(), &[0xCC]);
}

#[test]
fn builder_out_of_order_loads_correctly_to_vm() {
    let vm: Vm = ImageBuilder::new()
        .seek(ByteAddress::from(0x200))
        .write_word(0xBBBBBBBB)
        .seek(ByteAddress::from(0x000))
        .write_word(0xAAAAAAAA)
        .seek(ByteAddress::from(0x100))
        .write_word(0xCCCCCCCC)
        .build()
        .unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(0x000)), 0xAAAAAAAA);
    assert_eq!(vm.read_word(ByteAddress::from(0x100)), 0xCCCCCCCC);
    assert_eq!(vm.read_word(ByteAddress::from(0x200)), 0xBBBBBBBB);
}

#[test]
fn builder_out_of_order_overlap_detected() {
    let builder = ImageBuilder::new()
        .seek(ByteAddress::from(0x100))
        .write_word(0xAAAAAAAA)
        .seek(ByteAddress::from(0x000))
        .write_word(0xBBBBBBBB)
        .seek(ByteAddress::from(0x102)) // overlaps the write at 0x100
        .write_word(0xCCCCCCCC);
    assert!(matches!(
        builder.error(),
        Some(ImageBuildError::Overlap { .. })
    ));
}

#[test]
fn builder_build_into_vm() {
    let vm: Vm = ImageBuilder::new()
        .seek(ByteAddress::from(0x1000))
        .write_word(0xDEADBEEF)
        .build()
        .unwrap();
    assert_eq!(vm.read_word(ByteAddress::from(0x1000)), 0xDEADBEEF);
    // Unwritten memory is zero.
    assert_eq!(vm.read_word(ByteAddress::from(0x0000)), 0);
}

#[test]
fn builder_build_into_image() {
    let image: Image = ImageBuilder::new()
        .write_byte(0xAA)
        .seek(ByteAddress::from(0x100))
        .write_byte(0xBB)
        .build()
        .unwrap();
    let entries: Vec<_> = image.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address().value(), 0);
    assert_eq!(entries[0].data(), &[0xAA]);
    assert_eq!(entries[1].address().value(), 0x100);
    assert_eq!(entries[1].data(), &[0xBB]);
}

#[test]
fn builder_build_program_runs_correctly() {
    use crate::is::inst;
    let instructions = [
        inst::addi(1, 0, 10),
        inst::addi(2, 0, 20),
        inst::add(3, 1, 2),
        inst::jmp(0, 0),
    ];
    let mut vm: Vm = ImageBuilder::new()
        .write_instructions(&instructions)
        .build()
        .unwrap();
    vm.run_until_loop();
    assert_eq!(vm.register(3), 30);
}

#[test]
fn vm_to_image_to_vm() {
    let mut vm1 = Vm::new();
    vm1.write_word(ByteAddress::from(0x1000), 0xDEADBEEF);
    vm1.write_word(ByteAddress::from(0x2000), 0x12345678);

    let image = vm1.image();
    let vm2 = Vm::from(&image);

    assert_eq!(vm2.read_word(ByteAddress::from(0x1000)), 0xDEADBEEF);
    assert_eq!(vm2.read_word(ByteAddress::from(0x2000)), 0x12345678);
    // Unwritten addresses still zero.
    assert_eq!(vm2.read_word(ByteAddress::from(0x3000)), 0);
}

#[test]
fn vm_to_bytes_to_vm() {
    let mut vm1 = Vm::new();
    vm1.write_word(ByteAddress::from(0x1000), 0xDEADBEEF);
    vm1.write_word(ByteAddress::from(0x2000), 0x12345678);

    let mut buf = Vec::new();
    vm1.write_image_to(&mut buf).unwrap();

    let vm2 = Vm::from(buf.as_slice());
    assert_eq!(vm2.read_word(ByteAddress::from(0x1000)), 0xDEADBEEF);
    assert_eq!(vm2.read_word(ByteAddress::from(0x2000)), 0x12345678);
    assert_eq!(vm2.read_word(ByteAddress::from(0x3000)), 0);
}

#[test]
fn image_to_vm() {
    let mut image = Image::new();
    image.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3, 4]).unwrap());
    image.add(ImageEntry::new(ByteAddress::from(0x200), vec![5, 6, 7, 8]).unwrap());

    let vm = Vm::from(&image);
    assert_eq!(vm.read_byte(ByteAddress::from(0x100)), 1);
    assert_eq!(vm.read_byte(ByteAddress::from(0x103)), 4);
    assert_eq!(vm.read_byte(ByteAddress::from(0x200)), 5);
    assert_eq!(vm.read_byte(ByteAddress::from(0x203)), 8);
    assert_eq!(vm.read_byte(ByteAddress::from(0x000)), 0);
}

#[test]
fn builder_to_vm_to_image_to_bytes_to_vm() {
    use crate::is::inst;
    // Build a program with ImageBuilder.
    let instructions = [
        inst::addi(1, 0, 7),
        inst::addi(2, 0, 6),
        inst::add(3, 1, 2),
        inst::jmp(0, 0),
    ];
    let mut vm1: Vm = ImageBuilder::new()
        .write_instructions(&instructions)
        .seek(ByteAddress::from(0x1000))
        .write_word(0xCAFEBABE)
        .build()
        .unwrap();

    // Run the program.
    vm1.run_until_loop();
    assert_eq!(vm1.register(3), 13);

    // Serialize to bytes.
    let mut buf = Vec::new();
    vm1.write_image_to(&mut buf).unwrap();

    // Reload from bytes and verify state.
    let vm2 = Vm::from(buf.as_slice());
    assert_eq!(vm2.read_word(ByteAddress::from(0x1000)), 0xCAFEBABE);
    // Instructions should still be in memory.
    for (i, instruction) in instructions.iter().enumerate() {
        let byte_addr = ByteAddress::from(i as u32 * 4);
        assert_eq!(vm2.read_word(byte_addr), instruction.encode());
    }
}

#[test]
fn image_entries_vm_skips_empty_blocks() {
    let mut vm = Vm::new();
    // Only write to one block.
    vm.write_word(ByteAddress::from(0x0000_1000), 0xABCDABCD);

    let entries: Vec<_> = ImageEntries::from(&vm).collect();
    // Only one entry; the written block.
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0x0000_1000);
}

#[test]
fn image_entries_vm_trims_zero_edges() {
    let mut vm = Vm::new();
    // Write non-zero data in the middle of a block.
    vm.write_byte(ByteAddress::from(0x0000_0100), 0x00); // zero, should not anchor start
    vm.write_byte(ByteAddress::from(0x0000_0200), 0xAB); // non-zero start
    vm.write_byte(ByteAddress::from(0x0000_0300), 0xCD); // non-zero end
    vm.write_byte(ByteAddress::from(0x0000_0400), 0x00); // zero, should not anchor end

    let entries: Vec<_> = ImageEntries::from(&vm).collect();
    assert_eq!(entries.len(), 1);
    // Entry should start and end at the non-zero span.
    assert_eq!(entries[0].address().value(), 0x0000_0200);
    let data = entries[0].data();
    assert_eq!(*data.first().unwrap(), 0xAB);
    assert_eq!(*data.last().unwrap(), 0xCD);
    // The span between non-zero bytes is included (may contain zeros).
    assert_eq!(data.len(), 0x0300 - 0x0200 + 1);
}

#[test]
fn image_entries_vm_skips_io_blocks() {
    #[derive(Debug)]
    struct DummyIo;
    impl IoController for DummyIo {
        fn read_byte(&self, _: BlockOffset) -> u8 {
            0xFF
        }
        fn tick(&mut self) {}
        fn write_byte(&mut self, _: BlockOffset, _: u8) {}
    }

    let mut vm = Vm::new();
    vm.write_word(ByteAddress::from(0x0001_0000), 0x12345678);
    vm.set_block(BlockIndex::from(0u16), Block::with_controller(DummyIo));

    let entries: Vec<_> = ImageEntries::from(&vm).collect();
    // Only the memory block should appear, not the IO block.
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0x0001_0000);
}

#[test]
fn image_entries_bytes_skips_bad_offsets() {
    let mut buf = Vec::new();
    // Bad entry: end < start, no data bytes to consume.
    buf.extend(0x0000_1000_u32.to_be_bytes());
    buf.extend(0x0000_0000_u32.to_be_bytes());
    // Good entry following immediately after.
    buf.extend(0x0000_2000_u32.to_be_bytes());
    buf.extend(0x0000_2001_u32.to_be_bytes());
    buf.extend([0xAB, 0xCD]);

    let entries: Vec<_> = ImageEntries::from(buf.as_slice()).collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].address().value(), 0x2000);
    assert_eq!(entries[0].data(), &[0xAB, 0xCD]);
}

#[test]
fn image_entries_bytes_handles_truncated_input() {
    // A stream that cuts off mid-header should produce no entries.
    let buf = [0x00, 0x00, 0x10]; // only 3 bytes, not enough for a header
    let entries: Vec<_> = ImageEntries::from(buf.as_slice()).collect();
    assert!(entries.is_empty());
}

#[test]
fn image_to_bytes_to_image() {
    let mut original = Image::new();
    original.add(ImageEntry::new(ByteAddress::from(0x100), vec![1, 2, 3]).unwrap());
    original.add(ImageEntry::new(ByteAddress::from(0x200), vec![4, 5, 6]).unwrap());

    let mut buf = Vec::new();
    original.write_to(&mut buf).unwrap();

    let recovered = Image::from(buf.as_slice());
    let entries: Vec<_> = recovered.entries().collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address().value(), 0x100);
    assert_eq!(entries[0].data(), &[1, 2, 3]);
    assert_eq!(entries[1].address().value(), 0x200);
    assert_eq!(entries[1].data(), &[4, 5, 6]);
}
