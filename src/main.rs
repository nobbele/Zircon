use std::io::Cursor;

use zircon::{
    compile, print_error, print_errors,
    tokenizer::{tokenize, TokenType, TokenizerResult},
    CompileError, Error, MultiResult, Result,
};

fn main() -> Result<()> {
    // let contents = std::fs::read_to_string("main.zir").unwrap();

    let contents = r#"
def TargetAddress = $10

sub boot {
    ld A, $FF
    ld TargetAddress*, A
    ld $6000*, A

    jp boot
}
    "#;

    let TokenizerResult { tokens, lines } =
        tokenize(&mut Cursor::new(contents.as_bytes())).unwrap();

    let mut has_error = false;
    for error in tokens.iter().filter(|token| token.ty == TokenType::Error) {
        print_error(
            &contents,
            &lines,
            CompileError {
                message: "Failed to parse token".to_string(),
                span: error.span.clone(),
            },
        );
        println!();

        has_error = true;
    }

    if has_error {
        return Err(Error::Tokenizer);
    }

    let binary = match compile(&contents, &tokens) {
        MultiResult::Ok(binary) => binary,
        MultiResult::Err(errors) => {
            print_errors(&contents, &lines, errors, 1);

            return Err(Error::Compile);
        }
    };

    // let mut out_file = std::fs::OpenOptions::new()
    //     .write(true)
    //     .create(true)
    //     .truncate(true)
    //     .open("out.bin")
    //     .unwrap();

    // out_file.write_all(&binary).unwrap();
    println!("{:#04X?}", binary);

    Ok(())
}
