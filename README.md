# Alphabet

Alphabet is an educational tool designed to demonstrate how different "levels"
of programming languages behave down to the system level. It uses a simplified,
customizable virtual machine with a minimal instruction set so that the process
of translating high-level languages into machine code is easy to understand.

Alphabet uses a number of memory-mapped virtual IO devices which can be mixed
and matched to create a variety of interactive experiences on the machine.

Alphabet is designed to be approachable by programmers of a variety of skill
levels, and it will hopefully feature interactive walk-through tutorials
in the future.

## Virtual machine

AlphabetVM is Alphabet's dedicated virtual machine. It is a 32-bit
reduced instruction set computer with only two simple components: an
array of 24 registers, and a size-configurable all-purpose
main memory. It lacks a file system, operating system, cache layer, or other
devices which unnecessarily abstract from Alphabet's goal of being
an educational tool about machine translation.

AlphabetVM's CPU uses 24-bit virtual addresses which byte-address into
the main memory's *active page*. The user can configure how many
16 MiB pages (from 1 to 2^32, or virtually unlimited) of memory
they want the VM to have; this is constrained by the memory
of the device they're running on. The active page is controlled
by the page register.

AlphabetVM is big-endian so that multi-byte data is more human-readable.

### Registers

AlphabetVM has 18 general-purpose 32-bit CPU registers, named
`@0` through `@17`. This convention is preferred for using these registers:

- `@0`-`@7`: Call-clobbered registers (may be overwritten and must be
saved by caller).
- `@8`-`@15`: Call-preserved registers (must be preserved by a callee).
- `@16` (alias `@R`): Return value register.
- `@17` (alias `@C`): Counter register, preferred for loops (call-clobbered).

The remaining 6 registers are special-purpose and may be controlled by the CPU:

- `@MPG`: **Active page** register controls the page to which
virtual addresses map.
- `@CLK`: **Clock** register increments once after every instruction.
Note that some instructions which are aliases break down into
more than one instruction.
- `@EXE`: **Instruction address** register (or program counter) points
to the active instruction.
- `@RET`: **Return address** register saves the address after a `call`
instruction, and is jumped to with the `ret` instruction.
- `@STK`: **Stack pointer** register points to the top of the stack
and is controlled by `stk` instructions.
- `@CTL`: **Status control** register stores system configuration
andd status flags (see below).

### Status/configuration register

The layout of the `@CTL` register is as follows. The first 6 bits
are arithmetic result flags.

- Bit 0: **Result is 0** flag.
- Bit 1: **Result is negative** flag.
- Bit 2: **Carry** flag.
- Bit 3: **Overflow** flag.
- Bit 4: **Divide by 0** flag.
- Bit 5: Reserved but unused.

The next 2 bits are reserved for stack control.

- Bit 6: **Return address stack** flag. If enabled, the return address is
automatically pushed and popped from the stack (as below).
- Bit 7 is reserved but unused.

The next 8 bits map to registers `@8` through `@15` for *automatic
stack pushes and pops* on function calls. Bit 8 corresponds to `@8` and
bit 15 corresponds to `@15`, etc. If the bit is 1, the register
is automatically pushed at the `call` instruction and automatically
popped at the `ret` instruction.

The next 2 bits make up a **status value** which represents execution state.
Writing it can be used to control execution.

* Value of 0: Program start. Writing 0 restarts the program.
* Value of 1: Program executing. Write 2 to pause, 0 to restart.
* Value of 2: Program paused. `@EXE` will hold the value of the next
instruction to execute. Write 1 to unpause, 0 to restart.
* Value of 3: Program finished. Write 0 to restart.

The next 6 bits contain an arbitrary **status code** for a finished program.
A value of 0 communicates success, while anything else communicates error.

The remaining 8 bits are reserved but unused.

The value of the `@CTL` register, like all other registers, starts at 0.

### Instruction format

Instructions appear in 2 different formats.

(Register R often receives the result of the operation.)

**R-type** instructions are operations involving registers.
They are prefixed with `0`.

```
0XXXXX ----------- BBBBB AAAAA RRRRR
Opcode Unused      Register args.
6 bits 11 bits     15 bits (5 bits each)
```

**I-type** instructions are operations involving registers that
may have embedded 16-bit immediate values. They are prefixed
with `1`.

```
1XXXXX IIIIIIIIIIIIIIII AAAAA RRRRR
Opcode Immediate value  Register args.
6 bits 16 bits          10 bits (5 bits each)
```

See the full [list of instructions](docs/instructions.md) for details.

### Floating-point

AlphabetVM **does not support** floating-point numbers at a machine level.
Programmers must rely on software-implemented floating-point arithmetic,
which is included in standard libraries for the Beta language and upward.

## Languages

Alphabet supports 5 custom-made languages that resemble different "levels"
of real-world programming languages. In prototyping, the compilers and
assemblers will be built in, but bootstrapping is planned.

- **Alpha** is Alphabet's assembly language, it represents machine instructions
in a human-readable format, with a handful of instruction aliases which expand
to multiple instructions.
- **Beta** is Alphabet's **C**. It is a low-level language with manual memory management.
- **Gamma** is Alphabet's **C#**. It is a garbage-collected, object-oriented language
with inheritance, generics, polymorphism, and reflection.
- **Delta** is Alphabet's **Rust**. It is a low-level language that enforces memory safety
through compile-time rules.
- **Epsilon** is Alphabet's **Python**, its only interpreted language. (The interpreter
will have to be written in a different language).

## Virtual I/O devices

Virtualized input/output devices like keyboard and mouse input, text console input
and output, graphics output, and network communication
can be "plugged in" to AlphabetVM and mapped directly to physical memory. There is no
interrupt-based I/O; all devices are controlled using polling with load and
store operations for simplicity.

### Instances

AlphabetVM is like a self-contained program. Each **instance** of AlphabetVM is one
CPU, one program, and one data block (the latter two both stored in memory).
Eventually, Alphabet's assembler, compilers, and interpreters will themselves run
on AlphabetVM. Therefore, instances of AlphabetVM will be able to interface with each
other as virtual I/O devices for the purposes of reading programs and either
executing them or writing machine code on a second instance.

