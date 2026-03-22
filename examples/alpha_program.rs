use std::{fs::File, io::Read};

use alphabetvm::lang::{alpha::lex::Lexer, ascii::AsciiString};

fn main() {
    let mut file = File::open("programs/alpha/test.alpha").expect("failed to open file");
    let mut source = String::new();
    file.read_to_string(&mut source)
        .expect("failed to read file");
    let source = AsciiString::try_from(source).expect("file is not valid ASCII");

    let lexer = Lexer::lex_str(source);

    for token in lexer {
        match token {
            Ok(token) => println!("{:?} {}", token.token(), token.source()),
            Err(token_error) => eprintln!("{:?}", token_error.error),
        }
    }
}
