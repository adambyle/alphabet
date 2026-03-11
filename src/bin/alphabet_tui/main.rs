use alphabetvm::{ascii, lang::alpha::lex::Lexer};

fn main() {
    let source = ascii!("Bruh");
    let lexer = Lexer::lex_str(source);
}
