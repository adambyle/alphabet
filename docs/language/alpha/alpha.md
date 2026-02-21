# The Alpha language

The **Alpha** programming language is the dedicated
assembly language for the Alphabet VM. It is the
human-readable representation of Alphabet's
[machine instructions](../../instruction-set.md).

Alpha files have the `.alpha` file extension. More
than one Alpha source file may be assembled into a single
image. Be aware that all source files have their cursor
set to 0 by default.

Named symbols declared by labels and `.equ` directives
are shared between files.

Alpha assembly may fail in the following ways:

- An unrecognized instruction or directive name.
- Invalid instruction or directive syntax.
- Invalid register name.
- Invalid immediate value.
- Data would cross block boundaries.
- Data or instructions would be placed beyond
the last valid address (the address does not wrap).
- The use of the `.org` directive, or the
assembly of multiple files, would cause data or
instructions to overlap in memory.

## Cursor

The assembler tracks a *cursor* which marks where the next
output will go. It starts at 0 for each source file. The assembler
reads down the source file line by line, writing to memory and
moving the cursor as necessary.

When writing instructions, the assembler does three things:
- Word-align the cursor, rounding up if necessary.
- Write the encoded instruction to memory.
- Move the cursor forward by 4.

The cursor does not wrap around, and it is an error to try
to write to memory when the cursor is past the last valid
address.

Some directives also write to memory. Directives do not align the write
pointer, and they move the cursor forward by exactly the size
of the written data. It is an error for a single directive to
write across block boundaries.

The `.org` directive may move the cursor to any valid address.

## Basic syntax

All instructions and directives have some syntax rules in common:
- Whitespace is required between parts of an instruction or directive that
are not already separated by a symbol (like a comma).
- Excess whitespace is allowed before, after, or in the middle of instructions or directives.
Empty lines are allowed.
- Registers are formatted like `r#`, where `#` is 0-31. Single-digit register
names may be padded with a single 0. `ra` and `sp` are valid aliases for `r30`
and `r31`, respectively.

## Comments

The first `;` in each line begins a comment which extends to the end of the line.
Contents of comments are ignored.

## Immediate values

16-bit immediate values may be formatted according to the following rules:
- All numerical immediate values must be in the range (-2^15)..(2^16 - 1),
which encompasses all representable numbers in signed and unsigned formats. Be
aware that these values are translated to their binary representations, which
may differ from the way you have them written if your immediate implies a different
signedness than the instruction using it.
- Decimal immediates may be made up of only decimal digits.
- Binary, octal, and hexadecimal immediates may only be made up of their
respective digits, and must be prefixed with `0b`, `0o`, or `0x` respectively.
- Text immediates must be zero, one, or two quoted [ASCII characters](../../ascii.md),
which are translated to their 16-bit representation. Single characters fill the lower byte.
Omitted characters are zeroed.

32-bit and 8-bit immediate values, which are allowed in some directives,
also have the above rules apply, except that the valid range for
numerical immediate values and the number of characters in text immediates differ.

## Instructions

Alpha supports all of Alphabet's machine instructions and
has no additional pseudo-instructions. The name and behavior
of each instruction can be found [here](../../instruction-set.md).

The syntax of an instruction depends on its type.

### R-type

All **R-type instructions** list their operands in the order `R`, `A`, `B`,
separated by commas. An example instruction:

```alpha
add r3, r1, r2
```

This instruction adds the values of `r1` and `r2` and stores the result in `r3`.
The commas are mandatory and all 3 register arguments must be present.

The exception is the `noop` instruction, which has no arguments.

### I-type

The **I-type instructions** differ in syntax.

Instructions which use the immediate value as an address offset
(including the load, store, and `jmpr` instructions) have the following syntax.
The result register comes first, followed by the offset, followed by
the parenthesized name of the register argument storing the base
address.

```alpha
jmpr ra, 0x20(r1)
```

The `jmp`instruction is special in that it only names the result
register, and the second value is the offset.

```alpha
jmp ra, -4
```

The syntax of all other I-type instructions is similar to R-type instructions,
except that the immediate value replaces the last register argument.

```alpha
ori r1, r1, 0b1001
beq r2, r3, 80
```

## Directives

Directives are special commands to the assembler that begin with
the `.` symbol. The `.` symbol is a part of each directive's name;
it may not be followed by a space. Directives take comma-separated
arguments just like instructions. The following is a list of
valid directives.

|Directive|Behavior
|-|-
|`.equ <symbol>, <value>`|Declare the string of characters `symbol` to alias the specified immediate value (up to 32-bits). The symbol is valid wherever the aliased value would be a valid immediate value as an argument to an instruction or directive. `symbol` must consist only of alphanumeric characters and may not start with a digit. It is an error to declare the same symbol twice.
|`.org <address>`|Move the cursor to `address`, which must be a 32-bit unsigned immediate value.
|`.word <value>`|Write the provided 32-bit immediate value at the cursor.
|`.half <value>`|Write the provided 16-bit immediate value at the cursor.
|`.byte <value>`|Write the provided 8-bit immediate value at the cursor.
|`.string <value>`|Write the provided immediate text value (quoted) at the cursor. The maximum length is 2^16 characters.
|`.space <value>`|Move the cursor the specified number of bytes. `value` must be a 32-bit unsigned immediate value.

### Labels

Labels have separate syntax from other directives. They consist only of
a custom alphanumeric symbol (following the same rules as `.equ` symbols),
followed by a `:` symbol. A label symbol and a `.equ` symbol may not have
the same name.

Labels assign the value of the cursor at their location to the named symbol.
The label does not move the cursor. The symbol may be used in place of a 32-bit
immediate in instructions and directives.

Label symbols may also be used in `jmp`, `beq`, and `bne` instructions
in place of the offset. In these cases, the assembler substitutes
the offset of the label from the cursor at that instruction.
It is an error for this calculated offset to be out of the valid
16-bit-immediate range.

An example:

```alpha
addi r1, r0, 0x20

LOOP:
  beq r1, r0, END   ; Translates to beq r1, r0, 3 
  subi r1, r1, 1
  jmp r0, LOOP      ; Translates to jmp r0, -2

END:
  jmp r0, 0         ; Loop here endlessly.
```
