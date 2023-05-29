use crate::{
    tokenizer::{Token, TokenType},
    CompileError,
};

use super::Compiler;

impl<'a> Compiler<'a> {
    pub fn next_reset(&mut self) {
        while let Some(next) = self.peek() {
            if next.ty == TokenType::NewLine {
                break;
            }

            self.skip();
        }

        self.skip_line_sep();
    }

    pub fn skip_line_sep(&mut self) {
        while let Some(Token {
            ty: TokenType::NewLine,
            ..
        }) = self.peek()
        {
            self.skip();
        }
    }

    pub fn skip_comment_line(&mut self) {
        if let Some(Token {
            ty: TokenType::CommentLine,
            ..
        }) = self.remaining_tokens.get(0)
        {
            while let Some(next) = self.remaining_tokens.get(0) {
                if next.ty == TokenType::NewLine {
                    break;
                }

                self.skip();
            }
        }
    }

    pub fn peek(&mut self) -> Option<Token> {
        // TODO maybe skip comment line without actually calling .skip()? to keep immutability of this function.
        self.skip_comment_line();
        self.remaining_tokens.get(0).cloned()
    }

    pub fn next(&mut self) -> Option<Token> {
        self.skip_comment_line();
        let v = self.remaining_tokens.get(0).cloned();
        if let Some(v) = &v {
            self.latest_span = v.span.clone();
        }
        self.skip();
        v
    }

    pub fn skip(&mut self) {
        self.latest_span = self.remaining_tokens[0].span.clone();
        self.remaining_tokens = &self.remaining_tokens[1..];
    }

    pub fn peek_token_with_type(&mut self, target: TokenType) -> Result<Token, CompileError> {
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

    pub fn read_token_with_type(&mut self, target: TokenType) -> Result<Token, CompileError> {
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
}
