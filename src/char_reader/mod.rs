use std::io::{self, Read};

mod unicode;

pub struct CharReader<R: Read> {
    src: R,
    buffer: Box<[u8]>,
    pos: usize,
    len: usize,

    char_pos: Option<usize>,
    line: usize,
    col: Option<usize>,

    lines: Vec<usize>,
    was_newline: bool,
}

const DEFAULT_BUF_SIZE: usize = 5_000;

impl<R: Read> CharReader<R> {
    pub fn new(src: R) -> Self {
        let buf_size = DEFAULT_BUF_SIZE;
        let buffer = vec![0; buf_size].into_boxed_slice();
        Self {
            src,
            buffer,
            pos: 0,
            len: 0,

            char_pos: None,
            line: 0,
            col: None,

            lines: vec![0],
            was_newline: false,
        }
    }
    /// ensures there's at least one char in the buffer, and returns it with
    /// its size in bytes (or None if the underlying stream is finished).
    fn load_char(&mut self) -> io::Result<Option<(char, usize)>> {
        if self.pos >= self.len {
            // buffer empty
            self.len = self.src.read(&mut self.buffer)?;
            if self.len == 0 {
                return Ok(None);
            }
            self.pos = 0;
        }
        let b = self.buffer[self.pos];
        let char_size = unicode::utf8_char_width(b);
        if self.pos + char_size > self.len {
            // there's not enough bytes in buffer
            // we start by moving what we have at the start of the buffer to make some room
            self.buffer.copy_within(self.pos..self.len, 0);
            self.len -= self.pos;
            self.len += self.src.read(&mut self.buffer[self.len..])?;
            if self.len < char_size {
                // we may ignore one to 3 bytes not being correct UTF8 at the
                // very end of the stream (ie return None instead of an error)
                return Ok(None);
            }
            self.pos = 0;
        }
        let code_point = unicode::read_code_point(&self.buffer, self.pos, char_size);
        let c = std::char::from_u32(code_point)
            .ok_or(io::Error::new(io::ErrorKind::InvalidData, "Not UTF8"))?;

        Ok(Some((c, char_size)))
    }

    /// reads and returns the next char, or None in case of EOF
    pub fn next_char(&mut self) -> io::Result<Option<char>> {
        Ok(match self.load_char()? {
            Some(cw) => {
                self.pos += cw.1;

                self.col = Some(if let Some(col) = self.col { col + 1 } else { 0 });
                self.char_pos = Some(if let Some(pos) = self.char_pos {
                    pos + 1
                } else {
                    0
                });

                if self.was_newline {
                    self.line += 1;
                    self.col = Some(0);

                    self.lines.push(self.char_pos.unwrap());

                    self.was_newline = false;
                }

                if cw.0 == '\n' {
                    self.was_newline = true;
                }

                Some(cw.0)
            }
            None => None,
        })
    }

    /// returns the next char, but doesn't advance the cursor
    pub fn peek_char(&mut self) -> io::Result<Option<char>> {
        self.load_char().map(|cw| cw.map(|cw| cw.0))
    }

    pub fn col(&self) -> usize {
        self.col.unwrap()
    }

    pub fn peek_col(&self) -> usize {
        if self.was_newline {
            0
        } else {
            self.col.map(|col| col + 1).unwrap_or(0)
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn peek_line(&self) -> usize {
        if self.was_newline {
            self.line + 1
        } else {
            self.line
        }
    }

    pub fn pos(&self) -> usize {
        self.char_pos.unwrap()
    }

    pub fn peek_pos(&self) -> usize {
        self.char_pos.map(|pos| pos + 1).unwrap_or(0)
    }

    pub fn lines_consume(self) -> Vec<usize> {
        self.lines
    }
}

#[test]
pub fn char_reader_1() {
    use std::io::Cursor;

    let mut char_reader = CharReader::new(Cursor::new(b"\nHello\nWorld"));
    assert_eq!(char_reader.line(), 0);
    assert_eq!(char_reader.peek_col(), 0);
    assert_eq!(char_reader.peek_line(), 0);
    assert_eq!(char_reader.peek_pos(), 0);
    assert_eq!(char_reader.next_char().unwrap(), Some('\n'));
    assert_eq!(char_reader.line(), 0);
    assert_eq!(char_reader.col(), 0);
    assert_eq!(char_reader.pos(), 0);
    assert_eq!(char_reader.peek_col(), 0);
    assert_eq!(char_reader.peek_line(), 1);
    assert_eq!(char_reader.next_char().unwrap(), Some('H'));
    assert_eq!(char_reader.line(), 1);
    assert_eq!(char_reader.col(), 0);
    assert_eq!(char_reader.pos(), 1);
    assert_eq!(char_reader.peek_col(), 1);
    assert_eq!(char_reader.peek_line(), 1);
    assert_eq!(char_reader.next_char().unwrap(), Some('e'));
    assert_eq!(char_reader.line(), 1);
    assert_eq!(char_reader.col(), 1);
    assert_eq!(char_reader.pos(), 2);
    assert_eq!(char_reader.peek_col(), 2);
    assert_eq!(char_reader.peek_line(), 1);
    assert_eq!(char_reader.next_char().unwrap(), Some('l'));
    assert_eq!(char_reader.next_char().unwrap(), Some('l'));
    assert_eq!(char_reader.next_char().unwrap(), Some('o'));
    assert_eq!(char_reader.line(), 1);
    assert_eq!(char_reader.col(), 4);
    assert_eq!(char_reader.pos(), 5);
    assert_eq!(char_reader.next_char().unwrap(), Some('\n'));
    assert_eq!(char_reader.line(), 1);
    assert_eq!(char_reader.col(), 5);
    assert_eq!(char_reader.pos(), 6);
    assert_eq!(char_reader.next_char().unwrap(), Some('W'));
    assert_eq!(char_reader.line(), 2);
    assert_eq!(char_reader.col(), 0);
    assert_eq!(char_reader.pos(), 7);
    assert_eq!(char_reader.next_char().unwrap(), Some('o'));
    assert_eq!(char_reader.next_char().unwrap(), Some('r'));
    assert_eq!(char_reader.next_char().unwrap(), Some('l'));
    assert_eq!(char_reader.next_char().unwrap(), Some('d'));
    assert_eq!(char_reader.peek_col(), 5);
    assert_eq!(char_reader.peek_line(), 2);
    assert_eq!(char_reader.next_char().unwrap(), None);
}
