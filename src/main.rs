use std::fmt::{Display, Formatter};
use std::fs::read_to_string;
use std::iter::Peekable;

use anyhow::{bail, Context, Result};

#[derive(Debug)]
enum Expr {
    List(Box<[Expr]>),
    Symbol(String),
    Integer(i64),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::List(exprs) => {
                write!(f, "(")?;
                let mut exprs = exprs.iter().peekable();
                while let Some(expr) = exprs.next() {
                    expr.fmt(f)?;
                    if exprs.peek().is_some() {
                        write!(f, " ")?
                    }
                }
                write!(f, ")")?;
                Ok(())
            }
            Expr::Symbol(val) => val.fmt(f),
            Expr::Integer(val) => val.fmt(f),
        }
    }
}

enum Token {
    LeftParen,
    RightParen,
    Symbol(String),
    Integer(i64),
}

fn tokenize(stream: &mut Peekable<impl Iterator<Item = char>>) -> Result<Vec<Token>> {
    let mut out = vec![];

    let token_ends = "() ";

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
                    if token_ends.contains(*d) {
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

fn read_expr(tokens: &mut Peekable<impl Iterator<Item = Token>>) -> Result<Expr> {
    match tokens.next().context("input ended unexpectedly")? {
        Token::LeftParen => {
            // reading tail
            let mut contents = vec![];
            loop {
                match tokens.peek() {
                    Some(Token::RightParen) => {
                        tokens.next();
                        break Ok(Expr::List(contents.into_boxed_slice()));
                    }
                    _ => contents.push(read_expr(tokens)?),
                }
            }
        }
        Token::RightParen => {
            bail!("unexpected right parenthesis")
        }
        Token::Integer(val) => Ok(Expr::Integer(val)),
        Token::Symbol(val) => Ok(Expr::Symbol(val)),
    }
}

fn parse(stream: &mut impl Iterator<Item = char>) -> Result<Box<[Expr]>> {
    let mut tokens = tokenize(&mut stream.peekable())?.into_iter().peekable();
    let mut out = vec![];
    while tokens.peek().is_some() {
        out.push(read_expr(&mut tokens)?);
    }
    Ok(out.into_boxed_slice())
}

fn main() -> Result<()> {
    let contents = read_to_string("test.lang").context("unable to open source file")?;

    let exprs = parse(&mut contents.chars())?;

    for expr in exprs.iter() {
        println!("{}", expr);
    }

    Ok(())
}
