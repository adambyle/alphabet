//! Core Alphabet VM and instruction set implementation.
//!
//! This crate provides an API to instantiate a VM instance and manipulate
//! its memory and execute instructions arbitrarily. Currently,
//! the Alphabet VM can only run on a single thread, and I/O devices
//! must be synchronous.

pub mod is;
pub mod vm;
