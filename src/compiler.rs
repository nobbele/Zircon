use crate::{
    tokenizer::{Token, TokenType},
    CompileError, MultiResult, Span,
};

struct CompilerContext {
    address: usize,
    binary: Vec<u8>,
}

impl CompilerContext {
    pub fn reserve_min(&mut self, min_length: usize) {
        if min_length > self.binary.len() {
            self.binary
                .extend(std::iter::repeat(0).take(min_length - self.binary.len()));
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        let end_address = self.address + data.len();
        self.reserve_min(end_address);
        self.binary[self.address..end_address].copy_from_slice(data);
        self.address = end_address;
    }
}

struct Compiler<'a> {
    text: &'a str,
    remaining_tokens: &'a [Token],
    latest_span: Span,
    errors: Vec<CompileError>,

    /// Used to properly resolve late-declared identifiers.
    write_queue: Vec<Box<dyn FnOnce(&mut CompilerContext)>>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum ShortRegister {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
    I,
    R,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum LongRegister {
    AF,
    BC,
    DE,
    HL,
    PC,
    SP,

    IX,
    IY,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Register {
    Short(ShortRegister),
    Long(LongRegister),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum DataTarget {
    Register(Register),
    Address(u16),
    Immediate(u16),
}

impl<'a> Compiler<'a> {
    fn next_reset(&mut self) {
        while let Some(next) = self.peek() {
            if next.ty == TokenType::LineSeperator {
                break;
            }

            self.skip();
        }

        self.skip_line_sep();
    }

    fn skip_line_sep(&mut self) {
        while let Some(Token {
            ty: TokenType::LineSeperator,
            ..
        }) = self.peek()
        {
            self.skip();
        }
    }

    fn skip_comment_line(&mut self) {
        if let Some(Token {
            ty: TokenType::CommentLine,
            ..
        }) = self.remaining_tokens.get(0)
        {
            while let Some(next) = self.remaining_tokens.get(0) {
                if next.ty == TokenType::LineSeperator {
                    break;
                }

                self.skip();
            }
        }
    }

    fn peek(&mut self) -> Option<Token> {
        // TODO maybe skip comment line without actually calling .skip()? to keep immutability of this function.
        self.skip_comment_line();
        self.remaining_tokens.get(0).cloned()
    }

    fn next(&mut self) -> Option<Token> {
        self.skip_comment_line();
        let v = self.remaining_tokens.get(0).cloned();
        if let Some(v) = &v {
            self.latest_span = v.span.clone();
        }
        self.skip();
        v
    }

    fn skip(&mut self) {
        self.latest_span = self.remaining_tokens[0].span.clone();
        self.remaining_tokens = &self.remaining_tokens[1..];
    }

    fn peek_token_with_type(&mut self, target: TokenType) -> Result<Token, CompileError> {
        let Some(token) = self.peek() else {
            return Err(CompileError { message: "Expected identifier, found EOF".to_owned(), span: self.latest_span.clone() });
        };

        self.latest_span = token.span.clone();

        if token.ty != target {
            return Err(CompileError {
                message: format!("Expected {:?}, found {:?}", target, token.ty),
                span: token.span,
            });
        }

        Ok(token)
    }

    fn read_token_with_type(&mut self, target: TokenType) -> Result<Token, CompileError> {
        let Some(token) = self.next() else {
            return Err(CompileError { message: "Expected identifier, found EOF".to_owned(), span: self.latest_span.clone() });
        };

        self.latest_span = token.span.clone();

        if token.ty != target {
            return Err(CompileError {
                message: format!("Expected {:?}, found {:?}", target, token.ty),
                span: token.span,
            });
        }

        Ok(token)
    }

    fn write(&mut self, f: impl FnOnce(&mut CompilerContext) + 'static) {
        self.write_queue.push(Box::new(f));
    }

    fn read_ident(&mut self) -> Result<&str, CompileError> {
        let ident_token = self.read_token_with_type(TokenType::Identifier)?;
        Ok(ident_token.span.slice(self.text))
    }

    fn read_register(&mut self) -> Result<Register, CompileError> {
        let register = self.peek_register()?;
        self.skip();
        Ok(register)
    }

    fn peek_register(&mut self) -> Result<Register, CompileError> {
        let ident_token = self.peek_token_with_type(TokenType::Register)?;
        Ok(match ident_token.span.slice(self.text) {
            "A" => Register::Short(ShortRegister::A),
            "B" => Register::Short(ShortRegister::B),
            "C" => Register::Short(ShortRegister::C),
            "D" => Register::Short(ShortRegister::D),
            "E" => Register::Short(ShortRegister::E),
            "F" => Register::Short(ShortRegister::F),
            "H" => Register::Short(ShortRegister::H),
            "L" => Register::Short(ShortRegister::L),
            "I" => Register::Short(ShortRegister::I),
            "R" => Register::Short(ShortRegister::R),
            "AF" => Register::Long(LongRegister::AF),
            "BC" => Register::Long(LongRegister::BC),
            "DE" => Register::Long(LongRegister::DE),
            "HL" => Register::Long(LongRegister::HL),
            "PC" => Register::Long(LongRegister::PC),
            "SP" => Register::Long(LongRegister::SP),
            "IX" => Register::Long(LongRegister::IX),
            "IY" => Register::Long(LongRegister::IY),
            _ => panic!("Unimplemented register"),
        })
    }

    fn read_instruction(&mut self) -> Result<&str, CompileError> {
        let inst_token = self.read_token_with_type(TokenType::Instruction)?;
        Ok(inst_token.span.slice(self.text))
    }

    fn read_literal(&mut self) -> Result<u16, CompileError> {
        let literal = self.peek_literal()?;
        self.skip();
        Ok(literal)
    }

    fn peek_literal(&mut self) -> Result<u16, CompileError> {
        let Some(token) = self.peek() else {
            return Err(CompileError { message: "Expected identifier, found EOF".to_owned(), span: self.latest_span.clone() });
        };

        self.latest_span = token.span.clone();

        let value = match token.ty {
            TokenType::HexNumber => {
                let text = token.span.slice(self.text);
                let value_text = &text[1..];
                i32::from_str_radix(value_text, 16).map_err(|e| CompileError {
                    message: e.to_string(),
                    span: token.span.clone(),
                })?
            }
            _ => {
                return Err(CompileError {
                    message: format!("Expected a literal, found {:?}", token.ty),
                    span: token.span,
                });
            }
        };

        if value.abs() > 0xFFFF {
            return Err(CompileError {
                message: format!("Number '{}' is too big to fit into the A register", value),
                span: token.span,
            });
        }

        Ok(value as u16)
    }

    fn read_data_target(&mut self) -> Result<DataTarget, CompileError> {
        if let Ok(register) = self.peek_register() {
            assert_eq!(register, self.read_register()?);
            return Ok(DataTarget::Register(register));
        }

        if let Ok(literal) = self.peek_literal() {
            assert_eq!(literal, self.read_literal()?);

            let mut is_address = false;
            if let Some(next) = self.peek() {
                if next.ty == TokenType::Star {
                    self.skip();
                    is_address = true;
                }
            }

            return Ok(if is_address {
                DataTarget::Address(literal)
            } else {
                DataTarget::Immediate(literal)
            });
        }

        // TODO This error could be better for values that can *almost* be parsed, like a number can be parsed properly but fails due to size.
        Err(CompileError {
            message: "Invalid data target".to_owned(),
            span: self.latest_span.clone(),
        })
    }

    fn read_ld(&mut self) {
        let start_span = self.latest_span.clone();

        let to = match self.read_data_target() {
            Ok(name) => name,
            Err(e) => {
                self.errors.push(e);
                self.next_reset();
                return;
            }
        };
        let _to_span = self.latest_span.clone();

        if let Err(e) = self.read_token_with_type(TokenType::CommaSeperator) {
            self.errors.push(e);
            self.next_reset();
            return;
        }

        let from = match self.read_data_target() {
            Ok(value) => value,
            Err(e) => {
                self.errors.push(e);
                self.next_reset();
                return;
            }
        };
        let from_span = self.latest_span.clone();

        match (to, from) {
            (DataTarget::Register(Register::Short(short_reg)), DataTarget::Immediate(imm)) => {
                if imm > 0xFF {
                    self.errors.push(CompileError {
                        message: format!("Number '{}' is too big to fit into the A register", imm),
                        span: from_span.clone(),
                    });
                    self.next_reset();
                    return;
                }
                let value_byte = imm as u8;

                self.write(move |ctx| {
                    ctx.write(&[
                        match short_reg {
                            ShortRegister::A => 0x3E,
                            ShortRegister::B => 0x06,
                            ShortRegister::C => 0x0E,
                            ShortRegister::D => 0x16,
                            ShortRegister::E => 0x1E,
                            ShortRegister::H => 0x26,
                            ShortRegister::L => 0x2E,
                            _ => unimplemented!(),
                        },
                        value_byte,
                    ])
                });
            }
            (
                DataTarget::Address(addr),
                DataTarget::Register(Register::Short(ShortRegister::A)),
            ) => {
                let [addr_low, addr_high] = addr.to_le_bytes();
                self.write(move |ctx| ctx.write(&[0x32, addr_low, addr_high]));
            }
            _ => {
                let end_span = self.latest_span.clone();
                self.errors.push(CompileError {
                    message: format!("'ld' isn't implemented for {:?} <- {:?}", to, from),
                    // TODO make this span actually correct.
                    span: Span {
                        pos: start_span.pos.end..(end_span.pos.end + 1),
                        col: start_span.col.end..(end_span.col.end + 1),
                        line: start_span.line.end..(end_span.line.end + 1),
                    },
                });
                self.next_reset();
            }
        }
    }

    fn read_instruction_line(&mut self) {
        let inst = match self.read_instruction() {
            Ok(inst) => inst,
            Err(e) => {
                self.errors.push(e);
                self.next_reset();
                return;
            }
        }
        .to_owned();

        match inst.as_str() {
            "ld" => self.read_ld(),

            _ => {
                self.errors.push(CompileError {
                    message: format!("Unable to find mnemonic '{}'", inst),
                    span: self.latest_span.clone(),
                });
                self.next_reset();
            }
        }
    }

    fn read_block(&mut self) {
        if let Err(e) = self.read_token_with_type(TokenType::BlockStart) {
            self.errors.push(e);
            self.next_reset();
            return;
        }

        self.skip_line_sep();

        while let Some(peek_token) = self.peek() {
            if peek_token.ty == TokenType::BlockEnd {
                self.skip();
                break;
            }

            self.read_instruction_line();
            self.skip_line_sep();
        }
    }

    fn read_label_block(&mut self) {
        let specifier = self.next().unwrap();
        match specifier.span.slice(self.text) {
            "sub" => {
                let _name = match self.read_ident() {
                    Ok(name) => name,
                    Err(e) => {
                        self.errors.push(e);
                        self.next_reset();
                        return;
                    }
                };

                self.skip_line_sep();

                self.read_block();
            }
            ty => {
                self.errors.push(CompileError {
                    message: format!("Unimplemented specifier type '{}'", ty),
                    span: specifier.span,
                });
                self.next_reset();
            }
        }
    }

    fn compile_remaining(&mut self) {
        let token = self.peek().unwrap();
        match token.ty {
            TokenType::LabelSpecifier => {
                self.read_label_block();
            }
            _ => {
                self.errors.push(CompileError {
                    message: "Unexpected token".to_owned(),
                    span: token.span,
                });
                self.next_reset();
            }
        }
    }

    pub fn compile(mut self) -> MultiResult<Vec<u8>> {
        self.skip_line_sep();

        while self.remaining_tokens.len() > 0 {
            self.compile_remaining();
            self.skip_line_sep();
        }

        if self.errors.len() > 0 {
            return MultiResult::Err(self.errors);
        }

        let mut ctx = CompilerContext {
            address: 0,
            binary: Vec::new(),
        };

        for write in self.write_queue {
            write(&mut ctx);
        }

        MultiResult::Ok(ctx.binary)
    }
}

pub fn compile(text: &str, tokens: &[Token]) -> MultiResult<Vec<u8>> {
    Compiler {
        text,
        remaining_tokens: tokens,
        latest_span: Span::default(),
        errors: Vec::new(),
        write_queue: Vec::new(),
    }
    .compile()
}
