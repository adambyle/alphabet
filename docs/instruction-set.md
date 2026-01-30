# Alphabet VM instruction set

Alphabet supports thirty-six 32-bit instructions. They
are encoded either as **R-type** or **I-type** instructions.
This is a list of all instructions.

## R-type instructions

R-type instructions perform an operation on two
registers and store the result in a third.
They have opcodes `0x00`-`0x1F` and are
encoded in the following format:

```
OOOOOO  RRRRR AAAAA BBBBB ..........
Opcode  Registers (Result Operands)
6 bits  15 bits
```

Here is a list of R-type instructions. All instructions
store the result of the operation in the `R` register.

|Instruction|Opcode|Behavior
|-|-|-
|`noop`|`0x00`|No operation (`R` not written).
|`add`|`0x01`|Add the values of `A` and `B` (with wrapping).
|`sub`|`0x02`|Subtract the values of `A` and `B` (with wrapping).
|`shl`|`0x03`|Shift the value of `A` left by the wrapped value of `B`.
|`shr`|`0x04`|Logically shift the value of `A` right by the wrapped value of `B`.
|`sar`|`0x05`|Arithmetically shift the value of `A` right by the wrapped value of `B`.
|`and`|`0x06`|Bitwise and the values of `A` and `B`.
|`or`|`0x08`|Bitwise or the values of `A` and `B`.
|`xor`|`0x0A`|Bitwise exclusive-or the values of `A` and `B`.
|`slt`|`0x0C`|Result in 1 if the signed value of `A` is less than the signed value of `B`, otherwise 0.
|`sltu`|`0x0D`|Result in 1 if the unsigned value of `A` is less than the unsigned value of `B`, otherwise 0.

Additional notes and clarifications:
* All instructions except less-than comparison instructions
behave the same for signed and unsigned values.
* The bit shift amount operand of the shift instructions
is *wrapped* at 32, the number of bits
in the register.

## I-type instructions

I-type instructions perform an operation with respect to
the value of 1 or 2 registers and a 16-bit signed or unsigned
immediate value, sometimes storing the result in a second
register. The semantics of the `R` register value
depend on the particular instruction. The immediate value
may be used as an operand in an arithmetic operation or
as an address offset.

I-type instructions
have opcodes `0x20`-`0x3F` and are encoded in the following format:

```
OOOOOO  RRRRR AAAAA  IIIIIIIIIIIIIIII
Opcode  Registers    Immediate value
6 bits  10 bits      16 bits
```

Here is a list of I-type instructions. The usage
of the `R` register depends on the particular instruction. It may be
read from or written to. The immediate value may be signed or unsigned
depending on the particular instruction.

|Instruction|Opcode|`R` register|Immediate|Behavior
|-|-|-|-|-
|`addi`|`0x21`|Written|Unsigned|Add the immediate value to the value of `A`.
|`subi`|`0x22`|Written|Unsigned|Subtract the immediate value from the value of `A`.
|`shli`|`0x23`|Written|Unsigned|Shift the value of `A` left by the wrapped immediate value.
|`shri`|`0x24`|Written|Unsigned|Logically shift the value of `A` right by the wrapped immediate value.
|`sari`|`0x25`|Written|Unsigned|Arithmetically shift the value of `A` right by the wrapped immediate value.
|`andi`|`0x26`|Written|Bits|Bitwise and the immediate value with the lower 16 bits of `A`.
|`andui`|`0x27`|Written|Bits|Bitwise and the immediate value with the upper 16 bits of `A`.
|`ori`|`0x28`|Written|Bits|Bitwise or the immediate value with the lower 16 bits of `A`.
|`orui`|`0x29`|Written|Bits|Bitwise or the immediate value with the upper 16 bits of `A`.
|`xori`|`0x2A`|Written|Bits|Bitwise exclusive-or the immediate value with the lower 16 bits of `A`.
|`xorui`|`0x2B`|Written|Bits|Bitwise exclusive-or the immediate value with the upper 16 bits of `A`.
|`slti`|`0x2C`|Written|Signed|Result in 1 if the signed value of `A` is less than the immediate value, otherwise 0.
|`sltui`|`0x2D`|Written|Unsigned|Result in 1 if the unsigned value of `A` is less than the immediate value, otherwise 0.
|`ldw`|`0x31`|Written|Signed|Load a word of memory (see notes).
|`ldhw`|`0x32`|Written|Signed|Load a sign-extended half-word of memory (see notes).
|`ldhwu`|`0x33`|Written|Signed|Load a zero-extended half-word of memory (see notes).
|`ldb`|`0x34`|Written|Signed|Load a sign-extended byte of memory (see notes).
|`ldbu`|`0x35`|Written|Signed|Load a zero-extended byte of memory (see notes).
|`stw`|`0x36`|Read|Signed|Store to a word of memory (see notes).
|`sthw`|`0x37`|Read|Signed|Store a truncated value to a half-word of memory (see notes).
|`stb`|`0x38`|Read|Signed|Store a truncated value to a byte of memory (see notes).
|`jmp`|`0x39`|Written|Signed|Move the program counter by the immediate value, wrapping.
|`jmpr`|`0x3A`|Written|Signed|Move the program counter to the word address in `A` offset by the immediate value, wrapping.
|`beq`|`0x3B`|Read|Signed|Move the program counter by the immediate value, wrapping, if the values of `R` and `A` are equal.
|`bne`|`0x3C`|Read|Signed|Move the program counter by the immediate value, wrapping, if the values of `R` and `A` are not equal.

Additional notes and clarifications:
* Memory load and store instructions operate on the address at
the value of `A` offset by the signed immediate value, with wrapping.
The value to store is read from `R` for store instructions, and the value
loaded is written to `R` for load instructions.
* Jump-and-link instructions (`jmp` and `jmpr`) store the wrapped word-address
of the instruction following the jump instruction (the return address)
into `R`.
* Jump and branch instructions operate with *word addresses*, meaning the
byte address of the relevant instruction is 4 times the word address.
This applies to the immediate offset as well; an offset value of 1
is an offset of 4 bytes.
* The bit shift amount operand of the shift instructions
is *wrapped* at 32, the number of bits
in the register.
