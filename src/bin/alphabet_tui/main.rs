use std::{collections::HashSet, io, time::Duration};

use alphabet::{is, vm};
use crossterm::event;

use crate::state::{Cursor, Editing, State, Window};

mod state;
mod ui;

const CYCLES_PER_TICK: usize = 0x10000;
const PAGE_SIZE: u32 = 4 * 32;

// Debug.
fn write_r_type(
    state: &mut State,
    addr: u32,
    op: u8,
    r_result: usize,
    r_op_1: usize,
    r_op_2: usize,
) {
    let instruction = is::Instruction {
        op,
        payload: is::Payload {
            r_type: is::RType {
                r_result,
                r_op_1,
                r_op_2,
            },
        },
    };
    let word = instruction.encode();
    state.vm.write_word(addr, word);
}

fn write_i_type(state: &mut State, addr: u32, op: u8, r_result: usize, r_op: usize, imm: u16) {
    let instruction = is::Instruction {
        op,
        payload: is::Payload {
            i_type: is::IType {
                r_result,
                r_op,
                imm,
            },
        },
    };
    let word = instruction.encode();
    state.vm.write_word(addr, word);
}

fn main() -> io::Result<()> {
    // VM execution control.
    let mut state = State {
        vm: vm::Vm::new(),
        ui: ui::Ui::new()?,
        window: Window {
            mem_start: 0,
            is_io_block: false,
            cursor: Cursor::Register(1),
            editing: Editing::Off,
            do_jump: false,
        },
        running: false,
        breakpoints: HashSet::new(),
    };

    // Debug.

    // r1 = array pointer
    write_i_type(&mut state, 0x00, is::op::ADDI, 1, 0, 0x30);
    // r2 = accumulator = 0
    write_r_type(&mut state, 0x04, is::op::ADD, 2, 0, 0);
    // r3 = length of array in first byte
    write_i_type(&mut state, 0x08, is::op::LDBU, 3, 1, 0x00);
    // loop: read byte
    // jump to end if length is 0
    write_i_type(&mut state, 0x0C, is::op::BEQ, 0, 3, 6);
    // move array pointer
    write_i_type(&mut state, 0x10, is::op::ADDI, 1, 1, 0x01);
    // load value at array pointer in r4
    write_i_type(&mut state, 0x14, is::op::LDB, 4, 1, 0x00);
    // add to accumulator
    write_r_type(&mut state, 0x18, is::op::ADD, 2, 2, 4);
    // subtract 1 from length
    write_i_type(&mut state, 0x1C, is::op::SUBI, 3, 3, 0x01);
    // jump to top of loop
    write_i_type(&mut state, 0x20, is::op::JMP, 0, 0, -5i16 as u16);
    // loop infinitely
    write_i_type(&mut state, 0x24, is::op::JMP, 0, 0, 0);

    state.print()?;

    'main: loop {
        let mut any_events = false;

        if state.running {
            // Advance some number of cycles.
            for _ in 0..CYCLES_PER_TICK {
                state.vm.execute_and_advance();
                if state.breakpoints.contains(&state.vm.program_counter()) {
                    state.running = false;
                    any_events = true;
                    break;
                }
            }
        }

        // Resolve input events.
        while event::poll(Duration::ZERO)? {
            // Extract key events only.
            let event = event::read()?;
            let event::Event::Key(key_event) = event else {
                continue;
            };
            if !key_event.is_press() {
                continue;
            }
            let key_code = key_event.code;

            // Handle event.
            any_events = true;
            if let Editing::Value {
                ref mut nibbles,
                ref mut cursor,
                width,
            } = state.window.editing
            {
                // Accept input.
                if key_code.is_enter() {
                    // Modify relevant value.
                    let mut value: u32 = 0;
                    for nibble in *nibbles {
                        value <<= 4;
                        value |= nibble as u32;
                    }
                    match state.window.cursor {
                        Cursor::Register(r) => {
                            state.vm.write_register(r as usize, value);
                        }
                        Cursor::Memory { word, byte } => {
                            let addr = state.window.mem_start + word as u32 * 4 + byte as u32;
                            state.vm.write_byte(addr, value as u8);
                        }
                        Cursor::Address { is_block, .. } => {
                            let (value, keep_mask) = if is_block {
                                (value << 16, 0xFFFF)
                            } else {
                                (value, 0xFFFF << 16)
                            };
                            state.seek((state.window.mem_start & keep_mask) | value);
                        }
                    }
                    state.window.editing = Editing::Off;
                    continue;
                }
                if key_code.is_backspace() {
                    // Remove latest value.
                    for i in ((8 - width + 1)..8).rev() {
                        nibbles[i] = nibbles[i - 1]
                    }
                    nibbles[8 - width] = 0;
                    if *cursor > 0 {
                        *cursor -= 1;
                    }
                    continue;
                }

                // Process digit input. If failed, cancel.
                let digit = key_code.as_char().and_then(|c| c.to_digit(16));
                let Some(digit) = digit else {
                    state.window.editing = Editing::Off;
                    continue;
                };
                for i in (8 - width)..7 {
                    nibbles[i] = nibbles[i + 1];
                }
                nibbles[7] = digit as u8;
                if *cursor < width {
                    *cursor += 1;
                }
                continue;
            }

            if key_code.is_char('q') {
                break 'main;
            }
            if key_code.is_enter() {
                let width = match state.window.cursor {
                    Cursor::Register(_) => 8,
                    Cursor::Memory { .. } => 2,
                    Cursor::Address { .. } => 4,
                };
                state.window.editing = Editing::Value {
                    nibbles: [0; 8],
                    cursor: 0,
                    width: width,
                };
                continue;
            }
            if key_code.is_char('p') {
                if state.running {
                    state.running = false;
                    if state.window.do_jump {
                        state.show_pc();
                    }
                } else {
                    state.window.do_jump = state.is_pc_visible();
                    state.running = true;
                }
                continue;
            }
            if state.running {
                // Cannot perform any other actions
                // while running.
                continue;
            }
            if key_code.is_char('r') {
                state.vm.restart();
                state.seek(0);
                continue;
            }
            if key_code.is_char('s') {
                let do_jump = state.is_pc_visible();
                state.vm.execute_and_advance();
                if do_jump {
                    state.show_pc();
                }
                continue;
            }
            if key_code.is_char('b') {
                'bp: {
                    let Cursor::Memory { word, .. } = state.window.cursor else {
                        break 'bp;
                    };
                    let bp = (state.window.mem_start >> 2) + word as u32;
                    if !state.breakpoints.insert(bp) {
                        state.breakpoints.remove(&bp);
                    }
                }
                continue;
            }
            if key_code.is_char('<') || key_code.is_char(',') {
                let delta = if state.window.is_io_block {
                    vm::BLOCK_SIZE as u32
                } else {
                    PAGE_SIZE
                };
                state.seek(state.window.mem_start.wrapping_sub(delta));
                continue;
            }
            if key_code.is_char('>') || key_code.is_char('.') {
                let delta = if state.window.is_io_block {
                    vm::BLOCK_SIZE as u32
                } else {
                    PAGE_SIZE
                };
                state.seek(state.window.mem_start.wrapping_add(delta));
                continue;
            }
            if key_code.is_down() {
                state.window.cursor = match state.window.cursor {
                    Cursor::Memory { word, byte } => {
                        if word == 31 {
                            Cursor::Address {
                                is_block: true,
                                byte,
                            }
                        } else {
                            Cursor::Memory {
                                word: word + 1,
                                byte,
                            }
                        }
                    }
                    Cursor::Register(r) => Cursor::Register(r % 31 + 1),
                    Cursor::Address { byte, .. } => Cursor::Memory { word: 0, byte },
                };
                continue;
            }
            if key_code.is_up() {
                state.window.cursor = match state.window.cursor {
                    Cursor::Memory { word, byte } => {
                        if word == 0 {
                            Cursor::Address {
                                is_block: true,
                                byte,
                            }
                        } else {
                            Cursor::Memory {
                                word: word - 1,
                                byte,
                            }
                        }
                    }
                    Cursor::Register(r) => {
                        let r = if r == 1 { 31 } else { r - 1 };
                        Cursor::Register(r)
                    }
                    Cursor::Address { byte, .. } => Cursor::Memory { word: 31, byte },
                };
                continue;
            }
            if key_code.is_right() {
                state.window.cursor = match state.window.cursor {
                    Cursor::Memory { word, byte } => {
                        if byte == 3 {
                            Cursor::Register(word.max(1))
                        } else {
                            Cursor::Memory {
                                word,
                                byte: byte + 1,
                            }
                        }
                    }
                    Cursor::Register(r) => Cursor::Memory { word: r, byte: 0 },
                    Cursor::Address { is_block, byte } => Cursor::Address {
                        is_block: !is_block,
                        byte,
                    },
                };
                continue;
            }
            if key_code.is_left() {
                state.window.cursor = match state.window.cursor {
                    Cursor::Memory { word, byte } => {
                        if byte == 0 {
                            Cursor::Register(word.max(1))
                        } else {
                            Cursor::Memory {
                                word,
                                byte: byte - 1,
                            }
                        }
                    }
                    Cursor::Register(r) => Cursor::Memory { word: r, byte: 3 },
                    Cursor::Address { is_block, byte } => Cursor::Address {
                        is_block: !is_block,
                        byte,
                    },
                };
                continue;
            }
        }

        // Update display.
        if any_events {
            state.print()?;
        }
    }

    state.ui.close()?;

    Ok(())
}
