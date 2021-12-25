use std::iter::Peekable;

use anyhow::{bail, Result};

pub enum Token {
    LeftParen,
    RightParen,
    Symbol(String),
    Integer(i64),
}

pub fn tokenize(stream: &mut Peekable<impl Iterator<Item = char>>) -> Result<Vec<Token>> {
    let mut out = vec![];

    let token_ends = "()";

    loop {
        // single-char tokens
        match stream.peek() {
            Some(&'(') => {
                stream.next();
                out.push(Token::LeftParen);
            }
            Some(&')') => {
                stream.next();
                out.push(Token::RightParen);
            }
            Some(d) if d.is_whitespace() => {
                stream.next();
            }
            Some(d) if d.is_ascii() => {
                let mut s = String::new();
                while let Some(d) = stream.peek() {
                    if token_ends.contains(*d) || d.is_whitespace() {
                        break;
                    }
                    s.push(*d);
                    stream.next();
                }
                if let Ok(val) = s.parse() {
                    out.push(Token::Integer(val))
                } else {
                    out.push(Token::Symbol(s));
                }
            }
            Some(d) => {
                bail!("invalid character {}", d)
            }
            None => {
                break;
            }
        }
    }

    Ok(out)
}
