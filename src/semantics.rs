use anyhow::{bail, Context, Result};

use crate::parser::ParseExpr;

#[derive(Debug)]
pub enum Expr {
    VarAccess(String),
    ArithOp {
        operator: Operator,
        arg1: Box<Expr>,
        arg2: Box<Expr>,
    },
    Block(Box<[Expr]>),
    IfElse {
        pred: Box<Expr>,
        conseq: Box<Expr>,
        alt: Box<Expr>,
    },
    IntegerLiteral(i64),
}

#[derive(Copy, Clone, Debug)]
pub enum Operator {
    Add,
    Mul,
    Sub,
    Div,
}

impl Operator {
    fn is_variadic(&self) -> bool {
        match self {
            Operator::Add => true,
            Operator::Mul => true,
            Operator::Sub => false,
            Operator::Div => false,
        }
    }
}

fn nest_varargs(operator: Operator, mut args: Vec<Expr>) -> Result<Expr> {
    let first = args
        .pop()
        .context("arithmetic operations require at least one argument")?;
    Ok(if args.is_empty() {
        first
    } else {
        Expr::ArithOp {
            operator,
            arg1: Box::new(first),
            arg2: Box::new(nest_varargs(operator, args)?),
        }
    })
}

fn analyze_expr(expr: &ParseExpr) -> Result<Expr> {
    Ok(match expr {
        ParseExpr::Integer(val) => Expr::IntegerLiteral(*val),
        ParseExpr::List(call_expr) => {
            if let Some((ParseExpr::Symbol(operator), operands)) = call_expr.split_first() {
                let operator = match operator.as_str() {
                    "+" => Operator::Add,
                    "*" => Operator::Mul,
                    "-" => Operator::Sub,
                    "/" => Operator::Div,
                    _ => bail!("invalid operator in call expression"),
                };
                let mut operands = operands.iter().map(analyze_expr).collect::<Result<_>>()?;
                if operator.is_variadic() {
                    nest_varargs(operator, operands)?
                } else if operands.len() == 2 {
                    Expr::ArithOp {
                        operator,
                        arg1: Box::new(operands.pop().unwrap()),
                        arg2: Box::new(operands.pop().unwrap()),
                    }
                } else {
                    bail!("non-variadic arithops must have exactly two arguments")
                }
            } else {
                bail!("call expressions must have an operator")
            }
        }
        ParseExpr::Symbol(val) => Expr::VarAccess(val.to_string()),
    })
}

pub fn analyze(parsed: &[ParseExpr]) -> Result<Expr> {
    Ok(Expr::Block(
        parsed.iter().map(analyze_expr).collect::<Result<_>>()?,
    ))
}
