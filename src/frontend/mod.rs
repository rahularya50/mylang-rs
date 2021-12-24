use anyhow::Result;

mod lexer;
mod parser;

use self::lexer::tokenize;
use self::parser::read_expr;
pub use self::parser::ParseExpr;

pub fn parse(stream: &mut impl Iterator<Item = char>) -> Result<Box<[ParseExpr]>> {
    let mut tokens = tokenize(&mut stream.peekable())?.into_iter().peekable();
    let mut out = vec![];
    while tokens.peek().is_some() {
        out.push(read_expr(&mut tokens)?);
    }
    Ok(out.into_boxed_slice())
}
