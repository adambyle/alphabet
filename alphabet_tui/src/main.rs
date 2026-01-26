use std::{collections::HashSet, io, time::Duration};

use alphabet::{is, vm};
use crossterm::{event, style};

mod ui;

const CYCLES_PER_TICK: usize = 0x10000;
const PAGE_SIZE: u32 = 4 * 32;

enum Cursor {
    Register(u16),
    Memory { word: u16, byte: u16 },
}

enum Editing {
    Off,
    Value {
        nibbles: [u8; 8],
        cursor: usize,
        width: usize,
    },
}

struct Window {
    mem_start: u32,
    cursor: Cursor,
    editing: Editing,
}

struct State {
    vm: vm::Vm,
    ui: ui::Ui,
    window: Window,
    running: bool,
    breakpoints: HashSet<u32>,
}

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
            cursor: Cursor::Register(1),
            editing: Editing::Off,
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
    write_i_type(&mut state, 0x14, is::op::LDBU, 4, 1, 0x00);
    // add to accumulator
    write_r_type(&mut state, 0x18, is::op::ADD, 2, 2, 4);
    // subtract 1 from length
    write_i_type(&mut state, 0x1C, is::op::SUBI, 3, 3, 0x01);
    // jump to top of loop
    write_i_type(&mut state, 0x20, is::op::JMP, 0, 0, -5i16 as u16);
    // loop infinitely
    write_i_type(&mut state, 0x24, is::op::JMP, 0, 0, 0);

    print(&mut state)?;

    'main: loop {
        let mut any_events = false;

        if state.running {
            // Advance some number of cycles.
            for _ in 0..CYCLES_PER_TICK {
                state.vm.step_forward();
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
                    }
                    state.window.editing = Editing::Off;
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
                *cursor = (*cursor + 1).min(width);
                continue;
            }

            if key_code.is_char('q') {
                break 'main;
            }
            if key_code.is_enter() {
                let width = match state.window.cursor {
                    Cursor::Register(_) => 8,
                    Cursor::Memory { .. } => 2,
                };
                state.window.editing = Editing::Value {
                    nibbles: [0; 8],
                    cursor: 0,
                    width: width,
                };
                continue;
            }
            if key_code.is_char('p') {
                state.running = !state.running;
                continue;
            }
            if state.running {
                // Cannot perform any other actions
                // while running.
                continue;
            }
            if key_code.is_char('r') {
                state.vm.restart();
                continue;
            }
            if key_code.is_char('s') {
                state.vm.step_forward();
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
                let block_index = state.window.mem_start >> 16;
                let block = state.vm.get_block(block_index as u16);
                let sub = if let vm::Block::Io(_) = block {
                    vm::BLOCK_SIZE as u32
                } else {
                    PAGE_SIZE
                };
                state.window.mem_start = state.window.mem_start.wrapping_sub(sub);
                continue;
            }
            if key_code.is_char('>') || key_code.is_char('.') {
                let block_index = state.window.mem_start >> 16;
                let block = state.vm.get_block(block_index as u16);
                let sub = if let vm::Block::Io(_) = block {
                    vm::BLOCK_SIZE as u32
                } else {
                    PAGE_SIZE
                };
                state.window.mem_start = state.window.mem_start.wrapping_add(sub);
                continue;
            }
            if key_code.is_down() {
                match state.window.cursor {
                    Cursor::Memory { ref mut word, .. } => {
                        *word = (*word + 1) % 32;
                    }
                    Cursor::Register(ref mut r) => {
                        *r = *r % 31 + 1;
                    }
                };
                continue;
            }
            if key_code.is_up() {
                match state.window.cursor {
                    Cursor::Memory { ref mut word, .. } => {
                        *word = word.wrapping_sub(1) % 32;
                    }
                    Cursor::Register(ref mut r) => {
                        *r = if *r == 1 { 31 } else { *r - 1 };
                    }
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
                };
                continue;
            }
        }

        // Update display.
        if any_events {
            print(&mut state)?;
        }
    }

    state.ui.close()?;

    Ok(())
}

fn print(state: &mut State) -> io::Result<()> {
    // Controls.
    if state.running {
        show_running_controls(&mut state.ui)?;
    } else {
        show_stopped_controls(&mut state.ui)?;
    }

    let pc = state.vm.program_counter();
    let registers = state.vm.read_registers();

    // Program counter.
    state
        .ui
        .write_styled(4, 2, "Program counter", ui::WriteMode::Bold)?;
    state.ui.write(5, 2, &format!("{:08X}", pc))?;

    // Clear display.
    for row in 8..40 {
        state.ui.clear_row(row)?;
    }

    // Register values.
    state
        .ui
        .write_styled(7, 2, "Registers", ui::WriteMode::Bold)?;
    for (i, register) in registers.into_iter().enumerate().skip(1) {
        let i = i as u16;
        let highlight = if let Cursor::Register(r) = state.window.cursor {
            r == i
        } else {
            false
        };
        let style = if highlight && !state.running {
            ui::WriteMode::Highlighted
        } else {
            ui::WriteMode::Standard
        };
        state
            .ui
            .write_styled(9 + i, 2, &format!("r{i}"), ui::WriteMode::Bold)?;
        state
            .ui
            .write_styled(9 + i, 6, &format!("{register:08X}"), style)?;
        if highlight && !state.running {
            print_edit(9 + i, 6, state)?;
        }
    }

    // Current page memory values.
    let block_index = state.window.mem_start >> 16;
    let offset = state.window.mem_start & 0xFFFF;
    let block = state.vm.get_block(block_index as u16);
    state
        .ui
        .write_styled(7, 17, "Memory", ui::WriteMode::Bold)?;
    state.ui.write(7, 25, &format!("Block"))?;
    state.ui.write(7, 31, &format!("{:04X}", block_index))?;
    match block {
        vm::Block::Io(_) => {
            state.ui.write(7, 37, "I/O mapped")?;
        }
        vm::Block::Memory(mem) => {
            state.ui.write(7, 37, &format!("{:04X}", offset))?;
            state.ui.write(7, 41, "-")?;
            state
                .ui
                .write(7, 42, &format!("{:04X}", offset + (PAGE_SIZE - 1)))?;

            let mem = mem.read_all();
            let mem = &mem[offset as usize..(offset + PAGE_SIZE) as usize];
            for (word, bytes) in mem.chunks(4).enumerate() {
                let curr = (block_index << 14) + (offset >> 2) + word as u32;
                let is_pc = pc == curr && !state.running;
                let is_bp = state.breakpoints.contains(&curr);
                let style = if is_pc && is_bp {
                    ui::WriteMode::Blocked(style::Color::Red)
                } else if is_pc {
                    ui::WriteMode::Blocked(style::Color::Yellow)
                } else if is_bp {
                    ui::WriteMode::Colored(style::Color::Red)
                } else {
                    ui::WriteMode::Standard
                };
                state.ui.write_styled(
                    9 + word as u16,
                    17,
                    &format!("{:04X}", offset + 4 * word as u32),
                    style,
                )?;
                for (i, &byte) in bytes.iter().enumerate() {
                    let highlight = if let Cursor::Memory {
                        word: c_word,
                        byte: c_byte,
                    } = state.window.cursor
                    {
                        c_word == word as u16 && c_byte == i as u16
                    } else {
                        false
                    };
                    let style = if highlight && !state.running {
                        ui::WriteMode::Highlighted
                    } else {
                        ui::WriteMode::Standard
                    };
                    state.ui.write_styled(
                        9 + word as u16,
                        23 + 3 * i as u16,
                        &format!("{:02X}", byte),
                        style,
                    )?;
                    if highlight && !state.running {
                        print_edit(9 + word as u16, 23 + 3 * i as u16, state)?;
                    }
                }
            }
        }
    }
    state.ui.flush()?;
    Ok(())
}

fn print_edit(row: u16, col: u16, state: &mut State) -> io::Result<()> {
    let Editing::Value {
        nibbles,
        cursor,
        width,
    } = state.window.editing
    else {
        return Ok(());
    };

    let mut value: u32 = 0;
    for nibble in nibbles {
        value <<= 4;
        value |= nibble as u32;
    }

    state.ui.write_styled(
        row,
        col,
        &" ".repeat(width - cursor),
        ui::WriteMode::Blocked(style::Color::Cyan),
    )?;
    if cursor > 0 {
        state.ui.write_styled(
            row,
            col + width as u16 - cursor as u16,
            &format!("{:01$X}", value, cursor),
            ui::WriteMode::Blocked(style::Color::Cyan),
        )?;
    }

    Ok(())
}

fn show_running_controls(ui: &mut ui::Ui) -> io::Result<()> {
    ui.clear_row(0)?;
    ui.write(0, 0, "Running")?;

    ui.clear_row(1)?;

    ui.clear_row(2)?;
    ui.write_styled(2, 0, "Q", ui::WriteMode::Bold)?;
    ui.write(2, 2, "Quit")?;
    ui.write_styled(2, 19, "P", ui::WriteMode::Bold)?;
    ui.write(2, 21, "Pause program")?;
    Ok(())
}

fn show_stopped_controls(ui: &mut ui::Ui) -> io::Result<()> {
    ui.clear_row(0)?;
    ui.write(0, 0, "Stopped")?;

    ui.clear_row(1)?;
    ui.write_styled(1, 0, "< >", ui::WriteMode::Bold)?;
    ui.write(1, 4, "Change block")?;
    ui.write_styled(1, 18, "Enter", ui::WriteMode::Bold)?;
    ui.write(1, 24, "Edit cell")?;

    ui.clear_row(2)?;
    ui.write_styled(2, 0, "Q", ui::WriteMode::Bold)?;
    ui.write(2, 2, "Quit")?;
    ui.write_styled(2, 8, "R", ui::WriteMode::Bold)?;
    ui.write(2, 10, "Restart")?;
    ui.write_styled(2, 19, "P", ui::WriteMode::Bold)?;
    ui.write(2, 21, "Play program")?;
    ui.write_styled(2, 35, "S", ui::WriteMode::Bold)?;
    ui.write(2, 37, "Step forward")?;
    ui.write_styled(2, 51, "B", ui::WriteMode::Bold)?;
    ui.write(2, 53, "Toggle breakpoint")?;
    Ok(())
}
