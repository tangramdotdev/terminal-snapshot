use libghostty_sys as ghostty;
use std::{marker::PhantomData, ptr::{addr_of_mut, null, null_mut}};

pub struct Terminal {
    terminal: ghostty::GhosttyTerminal,
    render_state: ghostty::GhosttyRenderState,
}

pub type Options = ghostty::GhosttyTerminalOptions;

pub struct Snapshot<'a> {
    terminal: &'a mut Terminal,
}

pub struct Rows<'a> {
    row_iterator: ghostty::GhosttyRenderStateRowIterator,
    _snapshot: PhantomData<&'a mut Snapshot<'a>>,
}

pub struct Cells<'a> {
    cells: ghostty::GhosttyRenderStateRowCells,
    _row: PhantomData<&'a mut Rows<'a>>,
}

pub struct Cell {
    text: String,
}

#[derive(Copy, Clone, Debug)]
pub struct Error(i32);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self.0 {
            ghostty::GhosttyResult_GHOSTTY_SUCCESS => "success",
            ghostty::GhosttyResult_GHOSTTY_OUT_OF_MEMORY => "out of memory",
            ghostty::GhosttyResult_GHOSTTY_INVALID_VALUE => "invalid value",
            ghostty::GhosttyResult_GHOSTTY_OUT_OF_SPACE => "out of space",
            ghostty::GhosttyResult_GHOSTTY_NO_VALUE => "no value",
            _ => "unknown error",
        };
        write!(f, "{msg}")
    }
}

macro_rules! ghostty_call {
    ($e:expr) => {{
        let e = unsafe { $e };
        if e != ghostty::GhosttyResult_GHOSTTY_SUCCESS {
            return Err(Error(e));
        }
    }};
}

impl Terminal {
    pub fn new(options: Options) -> Result<Self, Error> {
        let mut terminal = null_mut();
        ghostty_call!(ghostty::ghostty_terminal_new(null(), addr_of_mut!(terminal), options));

        let mut render_state= null_mut();
        ghostty_call!(ghostty::ghostty_render_state_new(null(), addr_of_mut!(render_state)));

        Ok(Self { terminal, render_state })
    }

    pub fn write(&mut self, data: &[u8]) {
        unsafe { ghostty::ghostty_terminal_vt_write(self.terminal, data.as_ptr(), data.len()) }
    }

    pub fn snapshot(&mut self) -> Result<Snapshot<'_>, Error> {
        ghostty_call!(ghostty::ghostty_render_state_update(
            self.render_state,
            self.terminal
        ));
        Ok(Snapshot { terminal: self })
    }
}

impl<'a> Snapshot<'a> {
    pub fn rows(&self) -> Result<Rows<'a>, Error> {
        let mut row_iterator= null_mut();
        ghostty_call!(ghostty::ghostty_render_state_row_iterator_new(
            null(),
            addr_of_mut!(row_iterator),
        ));
        ghostty_call!(ghostty::ghostty_render_state_get(
            self.terminal.render_state,
            ghostty::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
            addr_of_mut!(row_iterator).cast(),
        ));
        Ok(Rows { row_iterator, _snapshot: PhantomData })
    }
}

impl<'a> Iterator for Rows<'a> {
    type Item = Result<Cells<'a>, Error>;
    fn next(&mut self) -> Option<Result<Cells<'a>, Error>> {
        let ok = unsafe { ghostty::ghostty_render_state_row_iterator_next(self.row_iterator) };
        if !ok {
            return None;
        }
        let mut cells = null_mut();
        let error = unsafe { ghostty::ghostty_render_state_row_cells_new(null(), addr_of_mut!(cells)) };
        if error != ghostty::GhosttyResult_GHOSTTY_SUCCESS {
            return Some(Err(Error(error)));
        }
        let error = unsafe {
            ghostty::ghostty_render_state_row_get(
                self.row_iterator,
                ghostty::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                addr_of_mut!(cells).cast(),
            )
        };
        if error != ghostty::GhosttyResult_GHOSTTY_SUCCESS {
            unsafe { ghostty::ghostty_render_state_row_cells_free(cells) };
            return Some(Err(Error(error)));
        }
        Some(Ok(Cells { cells, _row: PhantomData }))
    }
}

impl Iterator for Cells<'_> {
    type Item = Result<Cell, Error>;
    fn next(&mut self) -> Option<Result<Cell, Error>> {
        let ok = unsafe { ghostty::ghostty_render_state_row_cells_next(self.cells) };
        if !ok {
            return None;
        }
        let mut len = 0u32;
        let error = unsafe {
            ghostty::ghostty_render_state_row_cells_get(
                self.cells,
                ghostty::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                addr_of_mut!(len).cast(),
            )
        };
        if error != ghostty::GhosttyResult_GHOSTTY_SUCCESS {
            return Some(Err(Error(error)));
        }
        if len == 0 {
            return Some(Ok(Cell {
                text: String::new(),
            }));
        }
        let mut buf = vec![0u32; len as usize];
        let error = unsafe {
            ghostty::ghostty_render_state_row_cells_get(
                self.cells,
                ghostty::GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                buf.as_mut_ptr().cast()
            )
        };
        if error != ghostty::GhosttyResult_GHOSTTY_SUCCESS {
            return Some(Err(Error(error)));
        }
        Some(Ok(Cell {
            text: buf.into_iter().filter_map(char::from_u32).collect(),
        }))
    }
}

impl Cell {
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe {
            ghostty::ghostty_render_state_free(self.render_state);
            ghostty::ghostty_terminal_free(self.terminal);
        }
    }
}

impl Drop for Rows<'_> {
    fn drop(&mut self) {
        unsafe { ghostty::ghostty_render_state_row_iterator_free(self.row_iterator) }
    }
}

impl Drop for Cells<'_> {
    fn drop(&mut self) {
        unsafe { ghostty::ghostty_render_state_row_cells_free(self.cells) }
    }
}
