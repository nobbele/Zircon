use std::io::Cursor;

use zircon::{
    tokenizer::{tokenize, Token, TokenType, TokenizerResult},
    Span,
};

use TokenType::*;

#[test]
fn tokenizer_fail_1() {
    let TokenizerResult { tokens, lines: _ } = tokenize(&mut Cursor::new(
        br#"
sub boot {
    lda $FF??
    sta ($6000)

    jmp boot
    @fallthrough
}
"#,
    ))
    .unwrap();

    let error_tokens = tokens
        .into_iter()
        .filter(|tok| tok.ty == Error)
        .collect::<Vec<_>>();

    assert_eq!(
        error_tokens,
        vec![Token {
            ty: Error,
            span: Span {
                pos: 23..25,
                line: 2..3,
                col: 11..13
            }
        }]
    );
}

#[test]
fn tokenizer_keywords() {
    let TokenizerResult { tokens, lines: _ } = tokenize(&mut Cursor::new(
        br#"
ld A $FF
"#,
    ))
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token {
                ty: NewLine,
                span: Span {
                    pos: 0..1,
                    line: 0..1,
                    col: 0..1
                }
            },
            Token {
                ty: Instruction,
                span: Span {
                    pos: 1..3,
                    line: 1..2,
                    col: 0..2
                }
            },
            Token {
                ty: Register,
                span: Span {
                    pos: 4..5,
                    line: 1..2,
                    col: 3..4
                }
            },
            Token {
                ty: HexNumber,
                span: Span {
                    pos: 6..9,
                    line: 1..2,
                    col: 5..8
                }
            },
            Token {
                ty: NewLine,
                span: Span {
                    pos: 9..10,
                    line: 1..2,
                    col: 8..9
                }
            }
        ]
    );
}

#[test]
fn tokenizer_success() {
    let TokenizerResult { tokens, lines: _ } = tokenize(&mut Cursor::new(
        br#"
sub boot {
    jmp boot
    @fallthrough
}
"#,
    ))
    .unwrap();

    assert_eq!(
        tokens,
        vec![
            Token {
                ty: NewLine,
                span: Span {
                    pos: 0..1,
                    line: 0..1,
                    col: 0..1
                }
            },
            Token {
                ty: LabelSpecifier,
                span: Span {
                    pos: 1..4,
                    line: 1..2,
                    col: 0..3
                }
            },
            Token {
                ty: Identifier,
                span: Span {
                    pos: 5..9,
                    line: 1..2,
                    col: 4..8
                }
            },
            Token {
                ty: OpeningCurly,
                span: Span {
                    pos: 10..11,
                    line: 1..2,
                    col: 9..10
                }
            },
            Token {
                ty: NewLine,
                span: Span {
                    pos: 11..12,
                    line: 1..2,
                    col: 10..11
                }
            },
            Token {
                ty: Identifier,
                span: Span {
                    pos: 16..19,
                    line: 2..3,
                    col: 4..7
                }
            },
            Token {
                ty: Identifier,
                span: Span {
                    pos: 20..24,
                    line: 2..3,
                    col: 8..12
                }
            },
            Token {
                ty: NewLine,
                span: Span {
                    pos: 24..25,
                    line: 2..3,
                    col: 12..13
                }
            },
            Token {
                ty: At,
                span: Span {
                    pos: 29..30,
                    line: 3..4,
                    col: 4..5
                }
            },
            Token {
                ty: Identifier,
                span: Span {
                    pos: 30..41,
                    line: 3..4,
                    col: 5..16
                }
            },
            Token {
                ty: NewLine,
                span: Span {
                    pos: 41..42,
                    line: 3..4,
                    col: 16..17
                }
            },
            Token {
                ty: ClosingCurly,
                span: Span {
                    pos: 42..43,
                    line: 4..5,
                    col: 0..1
                }
            },
            Token {
                ty: NewLine,
                span: Span {
                    pos: 43..44,
                    line: 4..5,
                    col: 1..2
                }
            }
        ]
    );
}
