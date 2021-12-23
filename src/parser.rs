use std::fmt::{Display, Formatter};
use std::iter::Peekable;

use anyhow::{bail, Context, Result};

use crate::lexer::{tokenize, Token};

#[derive(Debug)]
pub enum Expr {
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

pub fn parse(stream: &mut impl Iterator<Item = char>) -> Result<Box<[Expr]>> {
    let mut tokens = tokenize(&mut stream.peekable())?.into_iter().peekable();
    let mut out = vec![];
    while tokens.peek().is_some() {
        out.push(read_expr(&mut tokens)?);
    }
    Ok(out.into_boxed_slice())
}
