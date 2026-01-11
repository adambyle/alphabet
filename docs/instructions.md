# List of machine instructions (Alpha language)

This is a complete list of machine instructions and their
counterparts in Alpha, including Alpha instruction aliases.

## Machine instructions

### `noop`

Syntax: `noop`
Opcode: `0x1F`

No operation.

### `mov` and `movi`

Syntax: `mov @R @A` and `movi @R IMM16` \
Opcode: `0x00` and `0x20`

Moves the value of `@A` or the immediate value `IMM16` to `@R`.
The immediate value is zero-extended to 32 bits.

### `movui`

Syntax: `movui @R IMM16` \
Opcode: `0x3F`

Moves the immediate value `IMM16` to the upper 16 bits of `@R`,
zeroing the lower 16 bits.

### `add` and `addi`

Syntax: `add @R @A @B` and `addi @R @A IMM16` \
Opcode: `0x01` and `0x21`

Adds the value of `@B` or the immediate value `IMM16` to `@A`,
storing the result in `@R`.

### `sub` and `subi`

Syntax: `sub @R @A @B` and `subi @R @A IMM16` \
Opcode: `0x02` and `0x22`

Subtracts the value of `@B` or the immediate value `IMM16` from `@A`,
storing the result in `@R`.

### `mul` and `muli`

Syntax: `mul @R @A @B` and `muli @R @A IMM16` \
Opcode: `0x03` and `0x23`

Multiplies the unsigned value of `@A` by the
unsigned value of `@B` or the immediate
value `IMM16`, storing the result in `@R`.

### `smul` and `smuli`

Syntax: `smul @R @A @B` and `smuli @R @A IMM16` \
Opcode: `0x04` and `0x24`

Multiplies the signed value of `@A` by the
signed value of `@B` or the immediate
value `IMM16`, storing the result in `@R`.

### `div` and `divi`

Syntax: `div @R @A @B` and `divi @R @A IMM16` \
Opcode: `0x05` and `0x25`

Divides the value of `@A` by the value of `@B` or the immediate
value `IMM16`, storing the floored result in `@R` and
discarding the remainder.

### `sdiv` and `sdivi`

Syntax: `sdiv @R @A @B` and `sdivi @R @A IMM16` \
Opcode: `0x06` and `0x26`

Divides the signed value of `@A` by the signed value
of `@B` or the immediate value `IMM16`, storing the floored
result in `@R` and discarding the remainder.

### `mod` and `modi`

Syntax: `mod @R @A @B` and `modi @R @A IMM16` \
Opcode: `0x07` and `0x27`

Divides the value of `@A` by the value of `@B` or the immediate
value `IMM16`, storing the remainder in `@R` (floored division leftover).

### `smod` and `smodi`

Syntax: `smod @R @A @B` and `smodi @R @A IMM16` \
Opcode: `0x08` and `0x28`

Divides the signed value of `@A` by the signed value of `@B`
or the immediate value `IMM16`, storing the remainder
in `@R` (floored division leftover).

### `not` and `noti`

Syntax `not @R @A` and `noti @R IMM16` \
Opcode: `0x09` and `0x29`

Inverts the bits of the value of `@A` or the immediate value `IMM16`
and stores the result in `@R`.

### `or` and `ori`

Syntax `or @R @A @B` and `ori @R @A IMM16` \
Opcode: `0x0A` and `0x2A`

Ors the bits of the value of `@A` with the value of `@B` or
the immediate value `IMM16` and stores the result in `@R`.

### `and` and `andi`

Syntax: `and @R @A @B` and `andi @R @A IMM16` \
Opcode: `0x0B` and `0x2B`

Ands the bits of the value of `@A` with the value of `@B` or
the immediate value `IMM16` and stores the result in `@R`.

### `xor` and `xori`

Syntax: `xor @R @A @B` and `xori @R @A IMM16` \
Opcode: `0x0C` and `0x2C`

Xors the bits of the value of `@A` with the value of `@B` or
the immediate value `IMM16` and stores the result in `@R`.

### `shl` and `shli`

Syntax: `shl @R @A @B` and `shli @R @A IMM16` \
Opcode: `0x0D` and `0x2D`

Shifts the bits of the value of `@A` left by the value of `@B`
or the immediate value `IMM16`, storing the result in `@R`.

### `shr` and `shri`

Syntax: `shr @R @A @B` and `shri @R @A IMM16` \
Opcode: `0x0E` and `0x2E`

Shifts the bits of the value of `@A` right by the value of `@B`
or the immediate value `IMM16`, inserting zeroes from the left
and storing the result in `@R`.

### `sar` and `sari`

Syntax: `sar @R @A @B` and `sari @R @A IMM16` \
Opcode: `0x0F` and `0x2F`

Shifts the signed value of `@A` right by the value of `@B`
or the immediate value `IMM16`, preserving the sign bit
and storing the result in `@R`.

### `ldw` and `ldwi`

Syntax: `ldw @R @A @B` and `ldwi @R @A IMM16` \
Opcode: `0x10` and `0x30`

Loads a word (32 bits) from the address `[@A + @B]` or `[@A + IMM16]`
into `@R`.

### `ldhw` and `ldhwi`

Syntax: `ldhw @R @A @B` and `ldhwi @R @A IMM16` \
Opcode: `0x11` and `0x31`

Loads a half-word (16 bits) from the address `[@A + @B]` or `[@A + IMM16]`
into `@R`, zero-extending to 32 bits.

### `ldb` and `ldbi`

Syntax: `ldb @R @A @B` and `ldbi @R @A IMM16` \
Opcode: `0x12` and `0x32`

Loads a byte (8 bits) from the address `[@A + @B]` or `[@A + IMM16]`
into `@R`, zero-extending to 32 bits.

### `sldhw` and `sldhwi`

Syntax: `sldhw @R @A @B` and `sldhwi @R @A IMM16` \
Opcode: `0x13` and `0x33`

Loads a half-word (16 bits) from the address `[@A + @B]` or `[@A + IMM16]`
into `@R`, sign-extending to 32 bits.

### `sldb` and `sldbi`

Syntax: `sldb @R @A @B` and `sldbi @R @A IMM16` \
Opcode: `0x14` and `0x34`

Loads a byte (8 bits) from the address `[@A + @B]` or `[@A + IMM16]`
into `@R`, sign-extending to 32 bits.

### `stw` and `stwi`

Syntax: `stw @R @A @B` and `stwi @R @A IMM16` \
Opcode: `0x15` and `0x35`

Stores a word (32 bits) from `@R` to the address `[@A + @B]` or `[@A + IMM16]`.

### `sthw` and `sthwi`

Syntax: `sthw @R @A @B` and `sthwi @R @A IMM16` \
Opcode: `0x16` and `0x36`

Stores a half-word (lower 16 bits of `@R`) to the address `[@A + @B]`
or `[@A + IMM16]`.

### `stb` and `stbi`

Syntax: `stb @R @A @B` and `stbi @R @A IMM16` \
Opcode: `0x17` and `0x37`

Stores a byte (lower 8 bits of `@R`) to the address `[@A + @B]`
or `[@A + IMM16]`.

### `cmp` and `cmpi`

Syntax: `cmp @A @B` and `cmpi @A IMM16` \
Opcode: `0x18` and `0x38`

Compares `@A` with `@B` or the immediate value `IMM16` by performing
a subtraction and setting flags in `@CTL`, but does not store the result.
Sets the zero flag if equal, negative flag if `@A < @B` (signed),
and carry flag if `@A < @B` (unsigned).

### `jmpeq` and `jmpeqi`

Syntax: `jmpeq @A` and `jmpeqi IMM16` \
Opcode: `0x19` and `0x39`

Jumps to the address in `@A` or the immediate address `IMM16` if the
zero flag is set (values are equal).

### `jmpne` and `jmpnei`

Syntax: `jmpne @A` and `jmpnei IMM16` \
Opcode: `0x1A` and `0x3A`

Jumps to the address in `@A` or the immediate address `IMM16` if the
zero flag is not set (values are not equal).

### `jmplt` and `jmplti`

Syntax: `jmplt @A` and `jmplti IMM16` \
Opcode: `0x1B` and `0x3B`

Jumps to the address in `@A` or the immediate address `IMM16` if the
negative flag is set (signed less than).

### `jmpgt` and `jmpgti`

Syntax: `jmpgt @A` and `jmpgti IMM16` \
Opcode: `0x1C` and `0x3C`

Jumps to the address in `@A` or the immediate address `IMM16` if both
the negative and zero flags are not set (signed greater than).

### `jmpult` and `jmpulti`

Syntax: `jmpult @A` and `jmpulti IMM16` \
Opcode: `0x1D` and `0x3D`

Jumps to the address in `@A` or the immediate address `IMM16` if the
carry flag is set (unsigned less than).

### `jmpugt` and `jmpugti`

Syntax: `jmpugt @A` and `jmpugti IMM16` \
Opcode: `0x1E` and `0x3E`

Jumps to the address in `@A` or the immediate address `IMM16` if both
the carry and zero flags are not set (unsigned greater than).

## Instruction aliases

### `mov! @R IMM32`

Alias for storing 32-bit immediate values into registers.

```
movui @R [Upper 16 bits]
ori @R @R [Lower 16 bits]
```

### `jmp! @A`

Alias for jumping directly to an instruction stored in a register.

```
mov @EXE @A
```

### `jmpi! IMM32`

Alias for jumping directly to an instruction at an immediate address.

If address is 16-bit:

```
movi @EXE IMM32
```

Otherwise:

```
movui @EXE [Upper 16 bits]
ori @EXE @EXE [Lower 16 bits]
```

### `jmpr! @A`

Alias for jumping a relative amount in the program.

```
add @EXE @EXE @A
```

### `jmpri! IMM16`

Alias for jumping a relative amount in the program by an immediate amount.
Jumping a 32-bit amount must be achieved using a
combination of `mov!` and `jmpr!`.

```
addi @EXE @EXE IMM16
```

### `call! @A`

```
mov @RET @EXE
addi @RET @RET 12
jmp! @A
```

### `calli! IMM32`

```
mov @RET @EXE
addi @RET @RET [12/16 depending on jmpi! expansion]
jmpi! IMM32
```

### `ret!`

```
jmp! @RET
```

### `push! @R`

```
stwi @R @STK 0
subi @STK @STK 4
```

### `pop! @R`

```
addi @STK @STK 4
ldwi @R @STK 0
```
