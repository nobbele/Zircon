use crate::{tokenizer::TokenType, CompileError, Span};

use super::{
    types::{DataTarget, Register, ShortRegister},
    Compiler,
};

macro_rules! try_return {
    ($self:expr, $call:expr) => {
        match $call {
            Ok(v) => v,
            Err(e) => {
                $self.errors.push(e);
                $self.next_reset();
                return;
            }
        }
    };
}

macro_rules! try_into_u8 {
    ($self:expr, $register:expr, $imm:expr, $span:expr) => {{
        if $imm > 0xFF {
            $self.errors.push(CompileError {
                message: format!(
                    "Number '{}' is too big to fit into the {:?} register",
                    $imm, $register
                ),
                span: $span.clone(),
            });
            $self.next_reset();
            return;
        }
        $imm as u8
    }};
}

impl<'a> Compiler<'a> {
    pub fn read_data_target(&mut self) -> Result<DataTarget, CompileError> {
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

        if let Ok(ident) = self.peek_ident() {
            let ident = ident.to_owned();
            assert_eq!(ident, self.read_ident()?);

            let mut is_address = false;
            if let Some(next) = self.peek() {
                if next.ty == TokenType::Star {
                    self.skip();
                    is_address = true;
                }
            }

            return Ok(if is_address {
                DataTarget::IdentifierAddress(ident)
            } else {
                DataTarget::IdentifierImmediate(ident)
            });
        }

        // TODO This error could be better for values that can *almost* be parsed, like a number can be parsed properly but fails due to size.
        Err(CompileError {
            message: "Invalid data target".to_owned(),
            span: self.latest_span.clone(),
        })
    }

    pub fn read_ld(&mut self) {
        let start_span = self.latest_span.clone();

        let to = try_return!(self, self.read_data_target());
        let _to_span = self.latest_span.clone();

        if let Err(e) = self.read_token_with_type(TokenType::Comma) {
            self.errors.push(e);
            self.next_reset();
            return;
        }

        let from = try_return!(self, self.read_data_target());
        let from_span = self.latest_span.clone();

        match (to.clone(), from.clone()) {
            (DataTarget::Register(Register::Short(short_reg)), DataTarget::Immediate(imm)) => {
                let imm = try_into_u8!(self, short_reg, imm, from_span);

                self.write(move |_ctx| {
                    [
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
                        imm,
                    ]
                });
            }
            (
                DataTarget::Address(addr),
                DataTarget::Register(Register::Short(ShortRegister::A)),
            ) => {
                let [addr_low, addr_high] = addr.to_le_bytes();
                self.write(move |_ctx| [0x32, addr_low, addr_high]);
            }
            (
                DataTarget::IdentifierAddress(ident),
                DataTarget::Register(Register::Short(ShortRegister::A)),
            ) => {
                self.write(move |ctx| {
                    let addr = ctx.get(&ident).unwrap();
                    let [addr_low, addr_high] = addr.to_le_bytes();
                    [0x32, addr_low, addr_high]
                });
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

    // TODO support `if not(Zero)`-like post-fixes
    pub fn read_jp(&mut self) {
        let target = try_return!(self, self.read_ident()).to_owned();
        self.write(move |ctx| {
            let addr = ctx.get(&target).unwrap();
            let [addr_low, addr_high] = addr.to_le_bytes();
            [0xC3, addr_low, addr_high]
        });
    }

    pub fn read_instruction_line(&mut self) {
        let inst = try_return!(self, self.read_instruction()).to_owned();

        match inst.as_str() {
            "ld" => self.read_ld(),
            "jp" => self.read_jp(),
            _ => {
                self.errors.push(CompileError {
                    message: format!("Unable to find mnemonic '{}'", inst),
                    span: self.latest_span.clone(),
                });
                self.next_reset();
            }
        }
    }

    pub fn read_block(&mut self) {
        try_return!(self, self.read_token_with_type(TokenType::OpeningCurly));

        self.skip_line_sep();

        while let Some(peek_token) = self.peek() {
            if peek_token.ty == TokenType::ClosingCurly {
                self.skip();
                break;
            }

            self.read_instruction_line();
            self.skip_line_sep();
        }
    }

    pub fn read_label_block(&mut self) {
        let specifier = self.next().unwrap();
        match specifier.span.slice(self.text) {
            "sub" => {
                let name = try_return!(self, self.read_ident()).to_owned();
                let name_span = self.latest_span.clone();

                let start_address = self.address;
                self.resolution({
                    let name = name.clone();
                    move |ctx| {
                        ctx.set(&name, start_address);
                        true
                    }
                });
                self.skip_line_sep();
                self.read_block();
                let end_address = self.address;

                if let Some(existing) = self.reserve_area(&name, start_address..end_address) {
                    self.errors.push(CompileError {
                        message: format!("Subroutine '{}' overlaps with '{}'", name, existing),
                        span: name_span,
                    });
                }
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

    pub fn read_data_decl(&mut self) {
        let tok = self.next().unwrap();
        match tok.span.slice(self.text) {
            "def" => {
                let name = try_return!(self, self.read_ident()).to_owned();

                if let Err(e) = self.read_token_with_type(TokenType::Equals) {
                    self.errors.push(e);
                    self.next_reset();
                    return;
                }

                // TODO more advanced expressions
                let value = try_return!(self, self.read_literal());

                self.resolution(move |ctx| {
                    ctx.set(&name, value);
                    true
                });
            }
            "rom" => {
                let name = try_return!(self, self.read_ident()).to_owned();

                if let Err(e) = self.read_token_with_type(TokenType::Colon) {
                    self.errors.push(e);
                    self.next_reset();
                    return;
                }

                let size = try_return!(self, self.read_literal());

                if let Err(e) = self.read_token_with_type(TokenType::Equals) {
                    self.errors.push(e);
                    self.next_reset();
                    return;
                }

                // TODO more advanced expressions
                let value_ident = try_return!(self, self.read_ident()).to_owned();

                let current_address = self.address;
                self.resolution(move |ctx| {
                    ctx.set(&name, current_address);
                    true
                });

                match size {
                    2 => self.write(move |ctx| {
                        let value = ctx.get(&value_ident).unwrap();
                        value.to_le_bytes()
                    }),
                    _ => {
                        self.errors.push(CompileError {
                            message: format!("Invalid data size '{}', expected 2", size),
                            span: tok.span,
                        });
                        self.next_reset();
                    }
                }
            }
            ty => {
                self.errors.push(CompileError {
                    message: format!("Unimplemented data declaration type '{}'", ty),
                    span: tok.span,
                });
                self.next_reset();
            }
        }
    }

    pub fn read_top_level_pragma(&mut self) {
        let _ = self.read_token_with_type(TokenType::At).unwrap();
        let directive = try_return!(self, self.read_ident()).to_owned();
        match directive.as_str() {
            "origin" => {
                let _ = try_return!(self, self.read_token_with_type(TokenType::OpeningParen));
                let new_address = try_return!(self, self.read_literal());
                self.set_address(new_address);
                let _ = try_return!(self, self.read_token_with_type(TokenType::ClosingParen));
            }
            _ => {
                self.errors.push(CompileError {
                    message: format!("Unknown top level directive '{}'", directive),
                    span: self.latest_span.clone(),
                });
                self.next_reset();
            }
        }
    }
}
