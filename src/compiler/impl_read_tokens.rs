use crate::{tokenizer::TokenType, CompileError};

use super::{
    types::{LongRegister, Register, ShortRegister},
    Compiler,
};

impl<'a> Compiler<'a> {
    pub fn read_ident(&mut self) -> Result<&str, CompileError> {
        let ident_token = self.read_token_with_type(TokenType::Identifier)?;
        Ok(ident_token.span.slice(self.text))
    }

    pub fn peek_ident(&mut self) -> Result<&str, CompileError> {
        let ident_token = self.peek_token_with_type(TokenType::Identifier)?;
        Ok(ident_token.span.slice(self.text))
    }

    pub fn read_register(&mut self) -> Result<Register, CompileError> {
        let register = self.peek_register()?;
        self.skip();
        Ok(register)
    }

    pub fn peek_register(&mut self) -> Result<Register, CompileError> {
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

    pub fn read_instruction(&mut self) -> Result<&str, CompileError> {
        let inst_token = self.read_token_with_type(TokenType::Instruction)?;
        Ok(inst_token.span.slice(self.text))
    }

    pub fn read_literal(&mut self) -> Result<u16, CompileError> {
        let literal = self.peek_literal()?;
        self.skip();
        Ok(literal)
    }

    pub fn peek_literal(&mut self) -> Result<u16, CompileError> {
        let Some(token) = self.peek() else {
            return Err(CompileError { message: "Expected identifier, found EOF".to_owned(), span: self.latest_span.clone() });
        };

        self.latest_span = token.span.clone();

        let value = match token.ty {
            TokenType::HexNumber => {
                let text = token.span.slice(self.text);
                let text = &text[1..];
                i32::from_str_radix(text, 16).map_err(|e| CompileError {
                    message: e.to_string(),
                    span: token.span.clone(),
                })?
            }
            TokenType::DecNumber => {
                let text = token.span.slice(self.text);
                i32::from_str_radix(text, 10).map_err(|e| CompileError {
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
}
