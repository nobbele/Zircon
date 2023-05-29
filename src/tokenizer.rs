use std::io::Read;

use crate::{CharReader, Result, Span};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TokenType {
    /// lda, sta, etc
    Instruction,
    /// sub
    LabelSpecifier,
    /// {
    OpeningCurly,
    /// }
    ClosingCurly,
    /// (
    OpeningParen,
    /// )
    ClosingParen,
    /// A, B, C, SP, PC, IX, etc
    Register,
    /// anything else that is alphanumeric-ish
    Identifier,
    /// $FF
    HexNumber,
    /// 430
    DecNumber,
    /// *
    Star,
    /// &
    Ampersand,
    /// \n
    NewLine,
    /// ,
    Comma,
    /// \/\/
    CommentLine,
    /// =
    Equals,
    /// @
    At,
    /// :
    Colon,
    /// def, const, var
    DataDeclaration,
    /// Unidentifiable tokens.
    Error,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Token {
    pub ty: TokenType,
    pub span: Span,
}

pub struct TokenizerResult {
    pub tokens: Vec<Token>,
    pub lines: Vec<usize>,
}

const INSTRUCTIONS: &[&str] = &["ld", "st", "jp"];
const LABEL_SPECIFIERS: &[&str] = &["sub"];
const REGISTERS: &[&str] = &[
    "pc", "sp", "a", "b", "c", "d", "e", "f", "h", "l", "ix", "iy", "i", "r",
];
const DATA_DECLARATIONS: &[&str] = &["def", "rom"];

pub fn tokenize(reader: &mut impl Read) -> Result<TokenizerResult> {
    let mut reader = CharReader::new(reader);

    let mut tokens = Vec::new();
    while let Some(char) = reader.peek_char()? {
        macro_rules! try_tokenize_single_char {
            ($c:expr, $ty:expr) => {
                if let Some(token) = try_tokenize_single_char(&mut reader, $c, $ty)? {
                    tokens.push(token);
                    continue;
                }
            };
        }

        try_tokenize_single_char!('\n', TokenType::NewLine);
        try_tokenize_single_char!('{', TokenType::OpeningCurly);
        try_tokenize_single_char!('}', TokenType::ClosingCurly);
        try_tokenize_single_char!('*', TokenType::Star);
        try_tokenize_single_char!('&', TokenType::Ampersand);
        try_tokenize_single_char!(',', TokenType::Comma);
        try_tokenize_single_char!('=', TokenType::Equals);
        try_tokenize_single_char!('@', TokenType::At);
        try_tokenize_single_char!('(', TokenType::OpeningParen);
        try_tokenize_single_char!(')', TokenType::ClosingParen);
        try_tokenize_single_char!(':', TokenType::Colon);

        if char.is_whitespace() {
            let _ = reader.next_char()?;
            continue;
        }

        if char == '/' {
            let _ = reader.next_char()?;

            let start_pos = reader.pos();
            let start_line = reader.line();
            let start_col = reader.col();

            let next = reader.next_char()?;

            let end_pos = reader.pos() + 1;
            let end_line = reader.line() + 1;
            let end_col = reader.col() + 1;

            let span = Span {
                pos: start_pos..end_pos,
                line: start_line..end_line,
                col: start_col..end_col,
            };

            if next != Some('/') {
                tokens.push(Token {
                    ty: TokenType::Error,
                    span,
                });
                continue;
            }

            tokens.push(Token {
                ty: TokenType::CommentLine,
                span,
            });
            continue;
        }

        if char == '$' {
            tokens.push(read_hex_literal(&mut reader)?);
            continue;
        }

        if char.is_numeric() {
            tokens.push(read_dec_literal(&mut reader)?);
            continue;
        }

        if char.is_alphabetic() {
            let (text, span) = read_ident(&mut reader)?;

            let text = text.to_lowercase();
            let text = text.as_str();

            let ty = if INSTRUCTIONS.contains(&text) {
                TokenType::Instruction
            } else if LABEL_SPECIFIERS.contains(&text) {
                TokenType::LabelSpecifier
            } else if REGISTERS.contains(&text) {
                TokenType::Register
            } else if DATA_DECLARATIONS.contains(&text) {
                TokenType::DataDeclaration
            } else {
                TokenType::Identifier
            };

            tokens.push(Token { ty, span });
            continue;
        }

        tokens.push(Token {
            ty: TokenType::Error,
            span: read_unidentifiable(&mut reader)?,
        });
    }

    Ok(TokenizerResult {
        tokens,
        lines: reader.lines_consume(),
    })
}

fn try_tokenize_single_char(
    reader: &mut CharReader<impl Read>,
    target: char,
    ty: TokenType,
) -> Result<Option<Token>> {
    if let Some(c) = reader.peek_char()? {
        if c == target {
            let _ = reader.next_char()?;
            return Ok(Some(Token {
                ty,
                span: Span {
                    pos: reader.pos()..(reader.pos() + 1),
                    line: reader.line()..(reader.line() + 1),
                    col: reader.col()..(reader.col() + 1),
                },
            }));
        }
    }

    Ok(None)
}

fn read_unidentifiable(reader: &mut CharReader<impl Read>) -> Result<Span> {
    let start_pos = reader.peek_pos();
    let start_line = reader.peek_line();
    let start_col = reader.peek_col();

    while let Some(c) = reader.peek_char()? {
        if c.is_whitespace() {
            break;
        }

        let _ = reader.next_char()?;
    }

    let end_pos = reader.pos() + 1;
    let end_line = reader.line() + 1;
    let end_col = reader.col() + 1;

    Ok(Span {
        pos: start_pos..end_pos,
        line: start_line..end_line,
        col: start_col..end_col,
    })
}

fn is_identifier_char(c: char, first: bool) -> bool {
    if c.is_alphanumeric() {
        return true;
    }

    if c == '_' {
        return true;
    }

    if !first && c.is_numeric() {
        return true;
    }

    false
}

fn read_ident(reader: &mut CharReader<impl Read>) -> Result<(String, Span)> {
    let start_pos = reader.peek_pos();
    let start_line = reader.peek_line();
    let start_col = reader.peek_col();

    let mut text = String::new();

    let first_char = reader.next_char()?.unwrap();
    assert!(is_identifier_char(first_char, true));
    text.push(first_char);

    while let Some(char) = reader.peek_char()? {
        if !is_identifier_char(char, false) {
            break;
        }

        text.push(char);

        // Advance the position.
        let _ = reader.next_char()?;
    }

    let end_pos = reader.pos() + 1;
    let end_line = reader.line() + 1;
    let end_col = reader.col() + 1;

    Ok((
        text,
        Span {
            pos: start_pos..end_pos,
            line: start_line..end_line,
            col: start_col..end_col,
        },
    ))
}

fn read_hex_literal(reader: &mut CharReader<impl Read>) -> Result<Token> {
    assert_eq!(reader.next_char()?.unwrap(), '$');

    let start_pos = reader.pos();
    let start_line = reader.line();
    let start_col = reader.col();

    while let Some(char) = reader.peek_char()? {
        if !char.is_ascii_hexdigit() {
            break;
        }

        // Advance the position.
        let _ = reader.next_char()?;
    }

    let end_pos = reader.pos() + 1;
    let end_line = reader.line() + 1;
    let end_col = reader.col() + 1;

    Ok(Token {
        ty: TokenType::HexNumber,
        span: Span {
            pos: start_pos..end_pos,
            line: start_line..end_line,
            col: start_col..end_col,
        },
    })
}

fn read_dec_literal(reader: &mut CharReader<impl Read>) -> Result<Token> {
    assert!(reader.next_char()?.unwrap().is_numeric());

    let start_pos = reader.pos();
    let start_line = reader.line();
    let start_col = reader.col();

    while let Some(char) = reader.peek_char()? {
        if !char.is_ascii_digit() {
            break;
        }

        // Advance the position.
        let _ = reader.next_char()?;
    }

    let end_pos = reader.pos() + 1;
    let end_line = reader.line() + 1;
    let end_col = reader.col() + 1;

    Ok(Token {
        ty: TokenType::DecNumber,
        span: Span {
            pos: start_pos..end_pos,
            line: start_line..end_line,
            col: start_col..end_col,
        },
    })
}
