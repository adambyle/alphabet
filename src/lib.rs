#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

//! Simple in-memory virtual machine for pedagogical projects.
//!
//! The Alphabet VM is extremely simplified virtual machine with a dedicated
//! instruction set, designed to prioritize _flexibility_ and _infallibility_.
//! - **Flexibility**: All machine instructions are *self-contained*, meaning
//! they only manipulate the register values or memory of the virtual machine.
//! Communication with the host environment is highly customizable through the
//! creation of memory-mapped I/O controllers.
//! - **Infallibility**: The Alphabet VM has no error states--it does not even
//! have a halt state! Invalid instructions are effectively no-ops, and
//! invalid reads and writes fail silently (and result in 0 if appropriate).
//!
//! You can read more about the motivation behind the Alphabet VM, as well
//! as details about Alphabet's instruction set and its behavior, on
//! [Github](https://github.com/adambyle/alphabet). The rest of this
//! documentation will describe how to work with the VM.
//!
//! # Hosts
//!
//! On its own, this crate is not very useful. Ideally, the VM is controlled
//! by a host, an application that both controls the execution of the VM and
//! facilitates communication with it using I/O controllers. A terminal
//! interface and a web interface are two hosts planned for creation.
//!
//! # Usage
//!
//! The [`Vm`] struct represents instances of an Alphabet virtual machine.
//! The API provides fine control over the system's program counter, registers,
//! and memory so that hosts can be as interactive as desired.
//!
//! The [`Image`] struct is used to pass around Alphabet programs and
//! data between media, such as files. It is a serialized representation
//! of VM memory, but it does not include register values or the program
//! counter, so it is appropriate for representing a program bundled with
//! its data.
//!
//! The [`ImageBuilder`] struct is a useful API for programatically creating
//! Alphabet images. An `ImageBuilder` can build into an `Image` or directly
//! into a `Vm`, or it can be written to a [writer](`std::io::Write`).
//!
//! # Example
//!
//! The following example creates a program using [`ImageBuilder`] that
//! sums the elements of an array.
//!
//! ```
//! use alphabet::{ImageBuilder, Vm, is::inst};
//!
//! fn main() {
//!     const LEN_ADDR: u32 = 0x30;
//!     const ARRAY_ADDR: u32 = 0x31;
//!     const R_ARRAY_PTR: usize = 1;
//!     const R_LEN: usize = 2;
//!     const R_SUM: usize = 3;
//!     const R_ARRAY_ELEM: usize = 4;
//!
//!     let array = vec![4u8, 7u8, -2i8 as u8];
//!     let len = array.len() as u8;
//!
//!     // Create a program that reads signed bytes
//!     // from an array and calculates their sum.
//!     let instructions = &[
//!         inst::ldbu(R_LEN, 0, LEN_ADDR as i16),
//!         inst::addi(R_ARRAY_PTR, 0, ARRAY_ADDR as u16),
//!         inst::add(R_SUM, 0, 0),
//!         inst::beq(R_LEN, 0, 6),
//!         inst::ldb(R_ARRAY_ELEM, R_ARRAY_PTR, 0),
//!         inst::add(R_SUM, R_SUM, R_ARRAY_ELEM),
//!         inst::addi(R_ARRAY_PTR, R_ARRAY_PTR, 1),
//!         inst::subi(R_LEN, R_LEN, 1),
//!         inst::jmp(0, -5),
//!         inst::jmp(0, 0),
//!     ];
//!     let builder = ImageBuilder::new()
//!         .write_instructions(instructions)
//!         .seek(LEN_ADDR)
//!         .write_byte(len)
//!         .seek(ARRAY_ADDR)
//!         .write_bytes(array);
//!     let mut vm: Vm = builder.build().expect("failed to build VM");
//!
//!     // Execute until the last instruction is reached.
//!     vm.run_until_loop();
//!
//!     // Print the sum.
//!     let sum = vm.register(R_SUM);
//!     println!("Sum: {sum}");
//! }
//! ```
//!
//! Notice that the `ImageBuilder` can be built directly to a [`Vm`] and that
//! both the instructions and data can be loaded in this way.
//!
//! The [`is::inst`] module provides functions to easily create instructions.
//!
//! Also notice the conventional way to detect that a program has ended. Because
//! the Alphabet VM has no halt state, infinite loops conventionally denote
//! the end of a program. The convenience method [`Vm::run_until_loop`] runs
//! until such an instruction is encountered.
//!
//! If we didn't want to run the program right away, the [`ImageBuilder::entries`]
//! method could be used to write the program and data to a file instead.

pub mod image;
pub mod is;
pub mod vm;

pub use image::{Image, ImageBuilder};
pub use is::{Instruction, Operation};
pub use vm::Vm;
