# Alphabet VM

*Alphabet* is an extremely simple virtual machine and
accompanying dedicated programming languages. It has a
minimal instruction set and simplified memory-mapped
I/O system into which users can "plug in" arbitrary devices.

The Alphabet virtual machine is intentionally barebones, having
32 general-purpose registers, and byte-addressed virtual memory
broken into 65 thousand "blocks" of 65 kilobytes. That's it!
Each block is either readable/writable memory or a memory-mapped
virtual I/O device. There are 32 instructions (with room for 64),
including variants that embed 16-bit immediate values. The goal of
this simplfied design is to make interacting with low-level architecture,
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
- [ ] Console window interface
- [ ] Sample virtual I/O devices
- [ ] Web interface using WASM
- [ ] Web I/O devices
- [ ] Code editor (for assembly and future languages)
- [ ] Debugger
- [ ] Dedicated language implementation
- [ ] Assembler and compiler bootstrapping
- [ ] Dedicated interpreted language (using native interpreter)

## Virtual machine

The Alphabet VM is a 32-bit system. It has 32 general-purpose
registers (some with conventions and aliases) and 4.3 gigabytes
of physical memory. It lacks an operating system, advanced memory
features like a cache layer or paging, or an exception/trap/interrupt
system. It also lacks instruction-level support for multiplication,
division, and floating-point arithmetic.

The VM's memory is byte-addressed. It is divided into blocks which are
2^16 bytes in size, which are allocated as needed behind the scenes or
mapped to virtual I/O devices. While alignment for half-word and full-word
reads and writes are not strictly enforced, reads and writes across
block boundaries will silently fail (reads return 0). Instructions
dealing with instruction addresses (i.e. jumping and branching instructions)
use word addresses instead of byte addresses, enforcing alignment.

### Virtual I/O devices

The VM supports memory-mapped software-driven I/O devices. Their behavior is
simple: a device mapped to a memory block controls the behavior of all reads
and writes to addresses within that block.

### Instruction set

The VM supports 32-bit instructions of two formats: **R-type** and **I-type**.

R-type instructions have the following format:

```
OOOOOO RRRRR AAAAA BBBBB ..........
Opcode Registers ()
```
