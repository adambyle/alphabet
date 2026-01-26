use std::io::{self, Write};

use crossterm::{
    cursor, execute, queue,
    style::{self, Stylize},
    terminal,
};

pub struct Ui {
    output: io::Stdout,
    cols: u16,
    rows: u16,
}

pub const DEFAULT_COLS: u16 = 80;
pub const DEFAULT_ROWS: u16 = 40;

pub enum WriteMode {
    Standard,
    Highlighted,
    Bold,
    Colored(style::Color),
    Blocked(style::Color),
}

impl Ui {
    pub fn new() -> io::Result<Self> {
        let mut ui = Self {
            output: io::stdout(),
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
        };

        // Switch screens.
        execute!(ui.output, terminal::EnterAlternateScreen)?;

        // Set window title.
        execute!(ui.output, terminal::SetTitle("AlphabetVM"))?;

        // Configure cursor.
        execute!(ui.output, cursor::Hide)?;

        ui.set_size(DEFAULT_ROWS, DEFAULT_COLS)?;
        ui.clear()?;

        Ok(ui)
    }

    pub fn close(mut self) -> io::Result<()> {
        self.clear()?;

        // Restore console.
        execute!(self.output, terminal::LeaveAlternateScreen)?;
        execute!(self.output, cursor::Show)?;

        Ok(())
    }

    pub fn clear(&mut self) -> io::Result<()> {
        queue!(self.output, terminal::Clear(terminal::ClearType::All))
    }

    pub fn clear_row(&mut self, row: u16) -> io::Result<()> {
        queue!(self.output, cursor::MoveToRow(row))?;
        queue!(
            self.output,
            terminal::Clear(terminal::ClearType::CurrentLine),
        )?;
        Ok(())
    }

    pub fn set_size(&mut self, rows: u16, cols: u16) -> io::Result<()> {
        self.rows = rows;
        self.cols = cols;
        execute!(self.output, terminal::SetSize(cols, rows))
    }

    fn seek(&mut self, row: u16, col: u16) -> io::Result<()> {
        queue!(self.output, cursor::MoveTo(col, row))
    }

    pub fn write_styled(
        &mut self,
        row: u16,
        col: u16,
        text: &str,
        mode: WriteMode,
    ) -> io::Result<()> {
        self.seek(row, col)?;
        let content = match mode {
            WriteMode::Standard => text.reset(),
            WriteMode::Highlighted => text.reverse(),
            WriteMode::Bold => text.bold(),
            WriteMode::Colored(color) => text.with(color).bold(),
            WriteMode::Blocked(color) => text.on(color),
        };
        queue!(self.output, style::PrintStyledContent(content))?;
        Ok(())
    }

    pub fn write(&mut self, row: u16, col: u16, text: &str) -> io::Result<()> {
        self.write_styled(row, col, text, WriteMode::Standard)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.output.flush()?;
        Ok(())
    }
}
