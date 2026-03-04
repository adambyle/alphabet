use super::*;

fn test_operation_encodings<const N: usize>(
    operations: [(&'static str, Operation, u8, u8); N],
    encoding: Encoding,
) {
    for (operation_name, operation, opcode_const, opcode_literal) in operations {
        let parsed_op = Operation::parse(operation_name)
            .expect(&format!("operation name {operation_name} failed to parse"));
        let newed_op = Operation::new(opcode_literal)
            .expect(&format!("opcode {opcode_literal} should be valid"));
        assert_eq!(parsed_op, operation);
        assert_eq!(newed_op, operation);
        assert_eq!(operation.name(), operation_name);
        assert_eq!(operation.encoding(), encoding);
        assert_eq!(operation.opcode(), opcode_const);
        assert_eq!(opcode_const, opcode_literal);
        assert!(Operation::is_valid_opcode(opcode_literal));
    }
}

#[test]
fn noop_operation() {
    let operations = [("noop", Operation::NOOP, Operation::NOOP_CODE, 0x00)];
    test_operation_encodings(operations, Encoding::Noop);
}

#[test]
fn r_type_operations() {
    let operations = [
        ("add", Operation::ADD, Operation::ADD_CODE, 0x01),
        ("sub", Operation::SUB, Operation::SUB_CODE, 0x02),
        ("shl", Operation::SHL, Operation::SHL_CODE, 0x03),
        ("shr", Operation::SHR, Operation::SHR_CODE, 0x04),
        ("sar", Operation::SAR, Operation::SAR_CODE, 0x05),
        ("and", Operation::AND, Operation::AND_CODE, 0x06),
        ("or", Operation::OR, Operation::OR_CODE, 0x08),
        ("xor", Operation::XOR, Operation::XOR_CODE, 0x0A),
        ("slt", Operation::SLT, Operation::SLT_CODE, 0x0C),
        ("sltu", Operation::SLTU, Operation::SLTU_CODE, 0x0D),
    ];
    test_operation_encodings(operations, Encoding::RType);
}

#[test]
fn i_type_operations() {
    let operations = [
        ("addi", Operation::ADDI, Operation::ADDI_CODE, 0x21),
        ("subi", Operation::SUBI, Operation::SUBI_CODE, 0x22),
        ("shli", Operation::SHLI, Operation::SHLI_CODE, 0x23),
        ("shri", Operation::SHRI, Operation::SHRI_CODE, 0x24),
        ("sari", Operation::SARI, Operation::SARI_CODE, 0x25),
        ("andi", Operation::ANDI, Operation::ANDI_CODE, 0x26),
        ("andui", Operation::ANDUI, Operation::ANDUI_CODE, 0x27),
        ("ori", Operation::ORI, Operation::ORI_CODE, 0x28),
        ("orui", Operation::ORUI, Operation::ORUI_CODE, 0x29),
        ("xori", Operation::XORI, Operation::XORI_CODE, 0x2A),
        ("xorui", Operation::XORUI, Operation::XORUI_CODE, 0x2B),
        ("slti", Operation::SLTI, Operation::SLTI_CODE, 0x2C),
        ("sltui", Operation::SLTUI, Operation::SLTUI_CODE, 0x2D),
        ("ldw", Operation::LDW, Operation::LDW_CODE, 0x31),
        ("ldhw", Operation::LDHW, Operation::LDHW_CODE, 0x32),
        ("ldhwu", Operation::LDHWU, Operation::LDHWU_CODE, 0x33),
        ("ldb", Operation::LDB, Operation::LDB_CODE, 0x34),
        ("ldbu", Operation::LDBU, Operation::LDBU_CODE, 0x35),
        ("stw", Operation::STW, Operation::STW_CODE, 0x36),
        ("sthw", Operation::STHW, Operation::STHW_CODE, 0x37),
        ("stb", Operation::STB, Operation::STB_CODE, 0x38),
        ("jmp", Operation::JMP, Operation::JMP_CODE, 0x39),
        ("jmpr", Operation::JMPR, Operation::JMPR_CODE, 0x3A),
        ("beq", Operation::BEQ, Operation::BEQ_CODE, 0x3B),
        ("bne", Operation::BNE, Operation::BNE_CODE, 0x3C),
    ];
    test_operation_encodings(operations, Encoding::IType);
}

#[test]
fn invalid_opcodes() {
    let invalid_ranges = [
        0x07..=0x07, // R-type binary logic
        0x09..=0x09,
        0x0B..=0x0B,
        0x0E..=0x1F, // R-type unused
        0x20..=0x20, // I-type unused no-op counterpart
        0x2E..=0x2F, // I-type unused
        0x30..=0x30, // Mem-type unused no-op counterpart
        0x3D..=0x3F, // Mem-type unused
    ];

    for opcode in invalid_ranges.into_iter().flatten() {
        assert!(!Operation::is_valid_opcode(opcode));
        assert!(Operation::new(opcode).is_none());
    }
}

#[test]
fn case_insensitive_parse_operation() {
    let lowercase_parsed = Operation::parse("add").unwrap();
    assert!(Operation::parse("ADD").is_some_and(|op| op == lowercase_parsed));
    assert!(Operation::parse("Add").is_some_and(|op| op == lowercase_parsed));
}

#[test]
fn all_operations() {
    let expected_count = 36;
    let count = Operation::all()
        .map(|op| assert!(Operation::is_valid_opcode(op.opcode())))
        .count();
    assert_eq!(count, expected_count);
}

#[test]
fn valid_payload_registers() {
    // Valid register indices.
    assert!(RTypePayload::new(0, 0, 0).is_ok());
    assert!(RTypePayload::new(31, 16, 3).is_ok());
    assert!(ITypePayload::new(0, 0, 0).is_ok());
    assert!(ITypePayload::new(31, 16, 0x300).is_ok());
    // One invalid register index.
    assert!(RTypePayload::new(32, 0, 0).is_err_and(|err| err.0 == 32));
    assert!(RTypePayload::new(0, 50, 0).is_err_and(|err| err.0 == 50));
    assert!(RTypePayload::new(0, 0, 72).is_err_and(|err| err.0 == 72));
    assert!(ITypePayload::new(89, 0, 0x1234).is_err_and(|err| err.0 == 89));
    assert!(ITypePayload::new(0, 32, 0x5678).is_err_and(|err| err.0 == 32));
    // One invalid reigster index and others are non-zero valid.
    assert!(RTypePayload::new(31, 32, 7).is_err_and(|err| err.0 == 32));
    assert!(RTypePayload::new(31, 8, 60).is_err_and(|err| err.0 == 60));
    assert!(ITypePayload::new(238, 1, 0x50).is_err_and(|err| err.0 == 238));
    assert!(ITypePayload::new(22, 90, 0x50).is_err_and(|err| err.0 == 90));
    // Many invalid register indices (no guarantees which is wrapped in error).
    assert!(RTypePayload::new(2, 93, 74).is_err());
    assert!(RTypePayload::new(81, 93, 74).is_err());
    assert!(ITypePayload::new(77, 99, 0x1F).is_err());
}

#[test]
fn r_type_payload_encoding() {
    fn round_trip(word: u32, payload: RTypePayload, r_r: usize, r_a: usize, r_b: usize) {
        const PAYLOAD_MASK: u32 = 0b00000011111111111111100000000000;
        let decoded = RTypePayload::decode(word);
        let encoded = decoded.encode();
        assert_eq!(decoded, payload);
        assert_eq!(encoded, word & PAYLOAD_MASK);
        assert_eq!(decoded.register_r_index(), r_r);
        assert_eq!(decoded.register_a_index(), r_a);
        assert_eq!(decoded.register_b_index(), r_b);
    }

    let tests = [
        (0x00000000, (0, 0, 0)), // Zero
        (0xFC0007FF, (0, 0, 0)), // Non-payload bits ignored
        (31 << 11, (0, 0, 31)),  // Register B
        (31 << 16, (0, 31, 0)),  // Register A
        (31 << 21, (31, 0, 0)),  // Register R
        ((1 << 21) | (2 << 16) | (3 << 11), (1, 2, 3)),
        ((31 << 21) | (31 << 16) | (31 << 11), (31, 31, 31)),
    ];

    for test in tests {
        let (word, (r_r, r_a, r_b)) = test;
        let payload = RTypePayload::new(r_r, r_a, r_b).expect("invalid register indices");
        round_trip(word, payload, r_r, r_a, r_b);
    }
}

#[test]
fn i_type_payload_encoding() {
    fn round_trip(word: u32, payload: ITypePayload, r_r: usize, r_a: usize, imm: u16) {
        const PAYLOAD_MASK: u32 = 0b00000011111111111111111111111111;
        let decoded = ITypePayload::decode(word);
        let encoded = decoded.encode();
        assert_eq!(decoded, payload);
        assert_eq!(encoded, word & PAYLOAD_MASK);
        assert_eq!(decoded.register_r_index(), r_r);
        assert_eq!(decoded.register_a_index(), r_a);
        assert_eq!(decoded.immediate_value(), imm);
    }

    let tests = [
        (0x00000000, (0, 0, 0)),  // Zero
        (0xFC000000, (0, 0, 0)),  // Non-payload bits ignored
        (0x1234, (0, 0, 0x1234)), // Immediate value
        (31 << 16, (0, 31, 0)),   // Register A
        (31 << 21, (31, 0, 0)),   // Register R
        ((16 << 21) | (21 << 16) | 0xDED, (16, 21, 0xDED)),
        ((31 << 21) | (31 << 16) | 0xFFFF, (31, 31, 0xFFFF)),
    ];

    for test in tests {
        let (word, (r_r, r_a, imm)) = test;
        let payload = ITypePayload::new(r_r, r_a, imm).expect("invalid register indices");
        round_trip(word, payload, r_r, r_a, imm);
    }
}

#[test]
fn instruction_encoding_mismatch() {
    let i_payload = Payload::new_i_type(0, 0, 0).unwrap();
    let r_payload = Payload::new_r_type(0, 0, 0).unwrap();
    for op in Operation::all() {
        if op == Operation::NOOP {
            continue;
        }
        let payload = if op.encoding() == Encoding::RType {
            i_payload.clone()
        } else {
            r_payload.clone()
        };
        let instruction = Instruction::new(op, payload);
        assert!(instruction.is_err_and(|err| err.encoding_error().is_some()));
    }
}

#[test]
fn instruction_payload_extractors() {
    let noop = Instruction::new(Operation::NOOP, Payload::new_noop()).unwrap();
    let r_inst = Instruction::new(Operation::ADD, Payload::new_r_type(1, 2, 3).unwrap()).unwrap();
    let i_inst =
        Instruction::new(Operation::ADDI, Payload::new_i_type(4, 5, 0x100).unwrap()).unwrap();

    assert!(noop.r_type_payload().is_none());
    assert!(noop.i_type_payload().is_none());

    assert!(
        r_inst
            .r_type_payload()
            .is_some_and(|p| p.register_r_index() == 1
                && p.register_a_index() == 2
                && p.register_b_index() == 3)
    );
    assert!(r_inst.i_type_payload().is_none());

    assert!(
        i_inst
            .i_type_payload()
            .is_some_and(|p| p.register_r_index() == 4
                && p.register_a_index() == 5
                && p.immediate_value() == 0x100)
    );
    assert!(i_inst.r_type_payload().is_none());
}

#[test]
fn instruction_encode_decode_roundtrip() {
    fn round_trip(instruction: Instruction) {
        let encoded = instruction.encode();
        let decoded = Instruction::decode(encoded).expect("decode should succeed");
        assert_eq!(decoded.operation(), instruction.operation());
        assert_eq!(decoded.payload(), instruction.payload());
    }

    round_trip(Instruction::new(Operation::NOOP, Payload::new_noop()).unwrap());

    // R-type: zero, mixed, max registers
    round_trip(Instruction::new(Operation::ADD, Payload::new_r_type(0, 0, 0).unwrap()).unwrap());
    round_trip(Instruction::new(Operation::SUB, Payload::new_r_type(1, 16, 31).unwrap()).unwrap());
    round_trip(Instruction::new(Operation::XOR, Payload::new_r_type(31, 31, 31).unwrap()).unwrap());

    // I-type: zero, mixed, max registers and immediate, negative offset
    round_trip(Instruction::new(Operation::ADDI, Payload::new_i_type(0, 0, 0).unwrap()).unwrap());
    round_trip(
        Instruction::new(Operation::LDW, Payload::new_i_type(4, 8, 0x00FF).unwrap()).unwrap(),
    );
    round_trip(
        Instruction::new(Operation::BNE, Payload::new_i_type(31, 31, 0xFFFF).unwrap()).unwrap(),
    );
    round_trip(
        Instruction::new(
            Operation::JMP,
            Payload::new_i_type(0, 0, (-4_i16) as u16).unwrap(),
        )
        .unwrap(),
    );
}

#[test]
fn instruction_decode_invalid_opcode() {
    for opcode in 0u8..=0x3F {
        let word = (opcode as u32) << Instruction::OPCODE_OFFSET;
        if Operation::is_valid_opcode(opcode) {
            assert!(Instruction::decode(word).is_ok());
        } else {
            assert!(
                Instruction::decode(word).is_err_and(|err| err.operation_error() == Some(opcode))
            );
        }
    }
}
