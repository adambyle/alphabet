use std::{collections::HashSet, io, time::Duration};

use alphabet::vm;
use crossterm::event;

mod ui;

const CYCLES_PER_TICK: usize = 0x10000;
const PAGE_SIZE: u32 = 4 * 32;

enum Cursor {
    Register(u16),
    Memory { word: u16, byte: u16 },
}

struct Window {
    mem_start: u32,
    cursor: Cursor,
}

fn main() -> io::Result<()> {
    let mut ui = ui::Ui::new()?;

    // VM execution control.
    let mut vm = vm::Vm::new();
    let mut running = false;
    let mut breakpoints = HashSet::<u32>::new();
    show_stopped_controls(&mut ui)?;
    let mut window = Window {
        mem_start: 0,
        cursor: Cursor::Register(1),
    };
    print_memory(&mut ui, &mut vm, &window)?;

    'main: loop {
        if running {
            // Advance some number of cycles.
            for _ in 0..CYCLES_PER_TICK {
                vm.step_forward();
                if breakpoints.contains(&vm.program_counter()) {
                    running = false;
                    break;
                }
            }
        }

        // Resolve input events.
        let mut any_events = false;
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
            if key_code.is_char('q') {
                break 'main;
            }
            if key_code.is_char('p') {
                running = !running;
                if running {
                    show_running_controls(&mut ui)?;
                } else {
                    show_stopped_controls(&mut ui)?;
                }
                continue;
            }
            if running {
                // Cannot perform any other actions
                // while running.
                continue;
            }
            if key_code.is_char('r') {
                vm.restart();
                continue;
            }
            if key_code.is_char('s') {
                vm.step_forward();
                continue;
            }
            if key_code.is_char('b') {
                let pc = vm.program_counter();
                if !breakpoints.insert(pc) {
                    breakpoints.remove(&pc);
                }
                continue;
            }
            if key_code.is_char('<') {
                let block_index = window.mem_start >> 16;
                let block = vm.get_block(block_index as u16);
                let sub = if let vm::Block::Io(_) = block {
                    vm::BLOCK_SIZE as u32
                } else {
                    PAGE_SIZE
                };
                window.mem_start = window.mem_start.wrapping_sub(sub);
                continue;
            }
            if key_code.is_char('>') {
                let block_index = window.mem_start >> 16;
                let block = vm.get_block(block_index as u16);
                let sub = if let vm::Block::Io(_) = block {
                    vm::BLOCK_SIZE as u32
                } else {
                    PAGE_SIZE
                };
                window.mem_start = window.mem_start.wrapping_add(sub);
                continue;
            }
            if key_code.is_down() {
                match window.cursor {
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
                match window.cursor {
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
                window.cursor = match window.cursor {
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
                window.cursor = match window.cursor {
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
        if !running && any_events {
            print_memory(&mut ui, &mut vm, &window)?;
        }
    }

    ui.close()?;

    Ok(())
}

fn print_memory(ui: &mut ui::Ui, vm: &mut vm::Vm, window: &Window) -> io::Result<()> {
    let pc = vm.program_counter();
    let registers = vm.read_registers();

    // Program counter.
    ui.write_styled(4, 2, "Program counter", ui::WriteMode::Bold)?;
    ui.write(5, 2, &format!("{:08X}", pc))?;

    // Clear display.
    for row in 8..40 {
        ui.clear_row(row)?;
    }

    // Register values.
    ui.write_styled(7, 2, "Registers", ui::WriteMode::Bold)?;
    for (i, register) in registers.into_iter().enumerate().skip(1) {
        let i = i as u16;
        let highlight = if let Cursor::Register(r) = window.cursor {
            r == i
        } else {
            false
        };
        let style = if highlight {
            ui::WriteMode::Highlighted
        } else {
            ui::WriteMode::Standard
        };
        ui.write_styled(9 + i, 2, &format!("r{i}"), ui::WriteMode::Bold)?;
        ui.write_styled(9 + i, 6, &format!("{register:08X}"), style)?;
    }

    // Current page memory values.
    let block_index = window.mem_start >> 16;
    let offset = window.mem_start & 0xFFFF;
    let block = vm.get_block(block_index as u16);
    ui.write_styled(7, 17, "Memory", ui::WriteMode::Bold)?;
    ui.write(7, 25, &format!("Block"))?;
    ui.write(7, 31, &format!("{:04X}", block_index))?;
    match block {
        vm::Block::Io(_) => {
            ui.write(7, 37, "I/O mapped")?;
        }
        vm::Block::Memory(mem) => {
            ui.write(7, 37, &format!("{:04X}", offset))?;
            ui.write(7, 41, "-")?;
            ui.write(7, 42, &format!("{:04X}", offset + (PAGE_SIZE - 1)))?;

            let mem = mem.read_all();
            let mem = &mem[offset as usize..(offset + PAGE_SIZE) as usize];
            for (word, bytes) in mem.chunks(4).enumerate() {
                ui.write(
                    9 + word as u16,
                    17,
                    &format!("{:04X}", offset + 4 * word as u32),
                )?;
                for (i, &byte) in bytes.iter().enumerate() {
                    let highlight = if let Cursor::Memory {
                        word: c_word,
                        byte: c_byte,
                    } = window.cursor
                    {
                        c_word == word as u16 && c_byte == i as u16
                    } else {
                        false
                    };
                    let style = if highlight {
                        ui::WriteMode::Highlighted
                    } else {
                        ui::WriteMode::Standard
                    };
                    ui.write_styled(
                        9 + word as u16,
                        23 + 3 * i as u16,
                        &format!("{:02X}", byte),
                        style,
                    )?;
                }
            }
        }
    }
    ui.flush()?;
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
