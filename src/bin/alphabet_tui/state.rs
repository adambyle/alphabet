use std::{collections::HashSet, io};

use alphabet::{is, vm};
use crossterm::style;

use crate::{PAGE_SIZE, ui};

pub enum Cursor {
    Register(u16),
    Memory { word: u16, byte: u16 },
    Address { is_block: bool, byte: u16 },
}

pub enum Editing {
    Off,
    Value {
        nibbles: [u8; 8],
        cursor: usize,
        width: usize,
    },
}

pub struct Window {
    pub mem_start: u32,
    pub is_io_block: bool,
    pub cursor: Cursor,
    pub editing: Editing,
    pub do_jump: bool,
}

pub struct State {
    pub vm: vm::Vm,
    pub ui: ui::Ui,
    pub window: Window,
    pub running: bool,
    pub breakpoints: HashSet<u32>,
}

impl State {
    pub fn seek(&mut self, mem_start: u32) {
        let block_index = mem_start >> 16;
        let block = self.vm.get_block(block_index as u16);
        let is_io = matches!(block, vm::Block::Io(_));
        let cutoff = if is_io { 16 } else { 7 };
        self.window.is_io_block = is_io;
        self.window.mem_start = mem_start >> cutoff << cutoff;
        if is_io {
            if let Cursor::Address {
                is_block: ref mut is_block @ false,
                ..
            } = self.window.cursor
            {
                *is_block = true;
            }
        }
    }

    pub fn is_pc_visible(&self) -> bool {
        let pc_addr = self.vm.program_counter() * 4;
        pc_addr >= self.window.mem_start && pc_addr < self.window.mem_start + PAGE_SIZE
    }

    pub fn show_pc(&mut self) {
        self.seek(self.vm.program_counter() * 4);
    }

    pub fn print_edit(&mut self, row: u16, col: u16) -> io::Result<()> {
        let Editing::Value {
            nibbles,
            cursor,
            width,
        } = self.window.editing
        else {
            return Ok(());
        };

        let mut value: u32 = 0;
        for nibble in nibbles {
            value <<= 4;
            value |= nibble as u32;
        }

        self.ui.write_styled(
            row,
            col,
            &" ".repeat(width - cursor),
            ui::WriteMode::Blocked(style::Color::Cyan),
        )?;
        if cursor > 0 {
            self.ui.write_styled(
                row,
                col + width as u16 - cursor as u16,
                &format!("{:01$X}", value, cursor),
                ui::WriteMode::Blocked(style::Color::Cyan),
            )?;
        }

        Ok(())
    }

    pub fn show_running_controls(&mut self) -> io::Result<()> {
        let ui = &mut self.ui;
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

    pub fn show_stopped_controls(&mut self) -> io::Result<()> {
        let ui = &mut self.ui;
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

    pub fn print(&mut self) -> io::Result<()> {
        // Controls.
        if self.running {
            self.show_running_controls()?;
        } else {
            self.show_stopped_controls()?;
        }

        let pc = self.vm.program_counter();
        let registers = self.vm.read_registers();

        // Program counter.
        self.ui
            .write_styled(4, 2, "Program counter", ui::WriteMode::Bold)?;
        self.ui.write(5, 2, &format!("{:08X}", pc))?;

        // Clear display.
        for row in 8..40 {
            self.ui.clear_row(row)?;
        }

        // Register values.
        self.ui
            .write_styled(7, 2, "Registers", ui::WriteMode::Bold)?;
        for (i, register) in registers.into_iter().enumerate().skip(1) {
            let i = i as u16;
            let highlight = if let Cursor::Register(r) = self.window.cursor {
                r == i
            } else {
                false
            };
            let style = if highlight && !self.running {
                ui::WriteMode::Highlighted
            } else {
                ui::WriteMode::Standard
            };
            let register_name = match i {
                30 => String::from("ra"),
                31 => String::from("sp"),
                i => format!("r{i}"),
            };
            self.ui
                .write_styled(9 + i, 2, &register_name, ui::WriteMode::Bold)?;
            self.ui
                .write_styled(9 + i, 6, &format!("{register:08X}"), style)?;
            if highlight && !self.running {
                self.print_edit(9 + i, 6)?;
            }
        }

        // Current page memory values.
        let block_index = self.window.mem_start >> 16;
        let offset = self.window.mem_start & 0xFFFF;
        self.ui.write_styled(7, 17, "Memory", ui::WriteMode::Bold)?;
        self.ui.write(7, 25, &format!("Block"))?;
        let is_block_cursor = matches!(self.window.cursor, Cursor::Address { is_block: true, .. });
        let is_offset_cursor = matches!(
            self.window.cursor,
            Cursor::Address {
                is_block: false,
                ..
            }
        );
        if is_block_cursor && !self.running {
            self.ui.write_styled(
                7,
                31,
                &format!("{:04X}", block_index),
                ui::WriteMode::Highlighted,
            )?;
            self.print_edit(7, 31)?;
        } else {
            self.ui.write(7, 31, &format!("{:04X}", block_index))?;
        }
        if is_offset_cursor && !self.running {
            self.ui.write_styled(
                7,
                37,
                &format!("{:04X}", offset),
                ui::WriteMode::Highlighted,
            )?;
            self.print_edit(7, 37)?;
        } else {
            self.ui.write(7, 37, &format!("{:04X}", offset))?;
        }
        let block = self.vm.get_block(block_index as u16);
        match block {
            vm::Block::Io(_) => {
                self.ui.write(7, 37, "I/O mapped")?;
            }
            vm::Block::Memory(mem) => {
                self.ui.write(7, 41, "-")?;
                self.ui
                    .write(7, 42, &format!("{:04X}", offset + (PAGE_SIZE - 1)))?;

                let mem = mem.read_all();
                let mem = &mem[offset as usize..(offset + PAGE_SIZE) as usize];
                for (word_offset, bytes) in mem.chunks(4).enumerate() {
                    let curr = (block_index << 14) + (offset >> 2) + word_offset as u32;
                    let is_pc = pc == curr && !self.running;
                    let is_bp = self.breakpoints.contains(&curr);
                    let style = if is_pc && is_bp {
                        ui::WriteMode::Blocked(style::Color::Red)
                    } else if is_pc {
                        ui::WriteMode::Blocked(style::Color::Yellow)
                    } else if is_bp {
                        ui::WriteMode::Colored(style::Color::Red)
                    } else {
                        ui::WriteMode::Standard
                    };
                    self.ui.write_styled(
                        9 + word_offset as u16,
                        17,
                        &format!("{:04X}", offset + 4 * word_offset as u32),
                        style,
                    )?;
                    for (i, &byte) in bytes.iter().enumerate() {
                        let highlight = if let Cursor::Memory {
                            word: c_word,
                            byte: c_byte,
                        } = self.window.cursor
                        {
                            c_word == word_offset as u16 && c_byte == i as u16
                        } else {
                            false
                        };
                        let style = if highlight && !self.running {
                            ui::WriteMode::Highlighted
                        } else {
                            ui::WriteMode::Standard
                        };
                        self.ui.write_styled(
                            9 + word_offset as u16,
                            23 + 3 * i as u16,
                            &format!("{:02X}", byte),
                            style,
                        )?;
                        if highlight && !self.running {
                            self.print_edit(9 + word_offset as u16, 23 + 3 * i as u16)?;
                        }
                    }

                    // Print representation of instruction.
                    let bytes = <[u8; 4]>::try_from(bytes).unwrap();
                    let word = u32::from_be_bytes(bytes);
                    if let Some(instruction) = is::Instruction::decode(word).as_string() {
                        if instruction != "noop" {
                            self.ui.write(9 + word_offset as u16, 36, &instruction)?;
                        }
                    }
                }
            }
        }
        self.ui.flush()?;
        Ok(())
    }
}
