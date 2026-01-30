# Alphabet VM

*Alphabet* is an extremely simple virtual machine. It has a
minimal instruction set and simplified memory-mapped
I/O system into which users can "plug in" arbitrary
virtual devices.

The Alphabet virtual machine is intentionally barebones, having
32 general-purpose registers and byte-addressed virtual memory
broken into 65 thousand "blocks" of 65 kilobytes. That's it!
Each block is either readable/writable memory or a memory-mapped
virtual I/O device. There are 36 instructions (with room for 64),
including variants that embed 16-bit immediate values. The goal of
this simplfied design is to make learning about interacting with low-level architecture,
machine code, and assembly language as easy as "ABC".

## Goals and roadmap

While Alphabet is mostly a personal project for exploring low-level
programming and programming language design, I'm hopeful it could
have real-world uses as a widely-available app. Alphabet will be
an educational tool and sandbox for interacting with assembly language
and machine code in a safe virtual environment.

Here's a brief project roadmap:

- [x] Virtual machine
- [x] Instruction set implementation
- [x] Terminal interface
- [ ] Assembler
- [ ] Sample virtual I/O devices
- [ ] Web interface using WASM
- [ ] Web I/O devices
- [ ] Web code editor (for assembly and future languages)
- [ ] Debugger
- [ ] Dedicated language implementation
- [ ] Assembler and compiler bootstrapping
- [ ] Dedicated interpreted language (using native interpreter)

## Virtual machine

The Alphabet VM is a 32-bit system. It has 32 registers and 4.3 gigabytes
of physical memory. It *lacks* an operating system, advanced memory
features like a cache layer or paging, and an exception/trap/interrupt
system. It also lacks instruction-level support for multiplication,
division, and floating-point arithmetic. It is big-endian.

### Memory

The VM's memory is byte-addressed. It is divided into blocks which are
2^16 bytes in size, which are allocated as needed behind the scenes or
mapped to virtual I/O devices. While alignment for half-word and full-word
reads and writes is not strictly enforced, reads and writes across
block boundaries will silently fail (reads return 0). Instructions
dealing with instruction addresses (i.e. jumping and branching instructions)
use word addresses instead of byte addresses, enforcing alignment.

It is encouraged to follow these memory conventions:

- Program begins at block `0x0000` (this is where the program counter starts).
- `0xF` blocks are memory-mapped I/O devices.
- The stack pointer should be initilized to `0xEFFFFFFC`
and grows downwards (toward zero).

### Virtual I/O devices

The VM supports memory-mapped software-driven I/O devices. Their behavior is
simple: a device mapped to a memory block controls the behavior of all reads
and writes to addresses within that block. These virtual I/O devices are
Alphabet's way of leveraging the capabilities of the host machine
or environment (such as reading mouse and keyboard input, drawing on a
display, or communicating across a network) without having to implement
those features as, say, system calls on the VM.

The planned terminal and web interfaces to Alphabet will come packaged with
a host of I/O devices that will allow users to create interactive experiences.

### Instruction set

The VM supports 32-bit instructions of two formats: **R-type** and **I-type**.
R-type instructions have opcodes `0x00`-`0x1F`, while I-type instructions
have opcodes `0x20`-`0x3F`.

R-type instructions have the following format:

```
OOOOOO  RRRRR AAAAA BBBBB ..........
Opcode  Registers (Result Operands)
6 bits  15 bits
```

I-type instructions have the following format:

```
OOOOOO  RRRRR AAAAA  IIIIIIIIIIIIIIII
Opcode  Registers    Immediate value
6 bits  10 bits      16 bits
```

Invalid opcodes are effectively no-ops.

See a [full list of instructions here](docs/instruction-set.md).

### Registers

Alphabet has 32 general-purpose 32-bit registers.
Reads to `r0` always return 0, and writes to `r0` are ignored.
The others may be used for any purpose, although implementations of
Alphabet's special programming languages will follow certain
conventions.

- `r1`-`r15` are **call-preserved** registers. If
a routine must make use of them, it is responsible for
restoring their values before returning control.
- `r16`-`r29` are **call-clobbered** registers. Routines
must push their values on the stack before a call if they
wish to preserve them.
- `r30` is the **return address**, alias `ra`.
- `r31` is the **stack pointer**, alias `sp`.

### Languages

Multiple languages will be designed specifically for the
Alphabet VM. 6 are planned right now:

- **Alpha** is an assembly language.
- **Beta** is a basic imperative language.
- **Gamma** is a structured imperative language with manual memory management.
- **Delta** is an object-oriented language with garbage collection.
- **Epsilon** is a functional language.
- **Zeta** is an interpreted scripting language.

The Zeta interpreter will itself be a program on the Alphabet VM
that receives a script from an input device.

### Images

VM memory state can be serialized and deserialized into **images**.
Images are the primary method of bundling a single program with
its data. Loading an image resets and overwrites all of the VM's
memory. Images do not save any data about I/O devices because they must
be platform agnostic. For example, a game with graphics may be exported
as an image, with the stipulation that it must be loaded on the Alphabet
web client with a pixel display device mapped to block `0xF000` and a keyboard
device to block `0xF001`. The platforms that run Alphabet may provide a means
to save and load configurations with their input devices, but images are the
most primitive means of representing VM state.

Images may be represented as files with the `.abc` extension. All values
in the image are big-endian. The layout of an image is a sequence of zero or more
**entries**. Each entry has the following layout:

- A 16-bit block index.
- A 16-bit byte offset.
- A 16-bit length value.
- The data in the provided block, with the provided length, at the provided offset.

There are a few things to note about the image layout if you are trying to build
an image manually or are targeting the Alphabet VM for compilation.

- If an entry is repeated for a block index, the last entry will overwrite
all of the ones before it. In other words, you cannot construct multiple regions
of memory within a block by having multiple entries for that block. This is
because all the memory in the block outside the provided region will be zeroed.
- If the offset plus the data length exceeds the size of a block (2^16), the
data that would be written past the end of the block is ignored.

If you're loading an image that was exported from the VM, you don't need to
think about these issues.

### Miscellaneous

Other important details/quirks about the Alphabet VM:

- The program counter will wrap around to 0 if it reaches
the end of memory.
- Instructions that change the program counter handle this
wrapping automatically.
- Programs should make no assumptions that registers and
memory will be be initialized to 0.
