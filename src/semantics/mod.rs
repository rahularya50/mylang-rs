use anyhow::{bail, Context, Result};

use crate::frontend::ParseExpr;

#[derive(Debug)]
pub enum Expr {
    VarDecl {
        name: String,
        value: Box<Expr>,
    },
    VarAssign {
        name: String,
        value: Box<Expr>,
    },
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
    Loop(Box<Expr>),
    Break,
    Continue,
    IntegerLiteral(i64),
    Noop,
    Return(Option<Box<Expr>>),
    Input,
}

#[derive(Copy, Clone, Debug)]
pub enum Operator {
    Add,
    Mul,
    Sub,
    Div,
}

impl Operator {
    fn is_variadic(self) -> bool {
        match self {
            Operator::Add | Operator::Mul => true,
            Operator::Sub | Operator::Div => false,
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

fn analyze_arithop(operator: Operator, operands: &[ParseExpr]) -> Result<Expr> {
    let mut operands = operands.iter().map(analyze_expr).collect::<Result<_>>()?;
    Ok(if operator.is_variadic() {
        nest_varargs(operator, operands)?
    } else if operands.len() == 2 {
        Expr::ArithOp {
            operator,
            arg1: Box::new(operands.pop().unwrap()),
            arg2: Box::new(operands.pop().unwrap()),
        }
    } else {
        bail!("non-variadic arithops must have exactly two arguments")
    })
}

fn analyze_if(operands: &[ParseExpr]) -> Result<Expr> {
    Ok(match operands {
        [pred, conseq] => Expr::IfElse {
            pred: Box::new(analyze_expr(pred)?),
            conseq: Box::new(analyze_expr(conseq)?),
            alt: Box::new(Expr::Noop),
        },
        [pred, conseq, alt] => Expr::IfElse {
            pred: Box::new(analyze_expr(pred)?),
            conseq: Box::new(analyze_expr(conseq)?),
            alt: Box::new(analyze_expr(alt)?),
        },
        _ => bail!("if statements must have either two or three arguments"),
    })
}

fn analyze_define(operands: &[ParseExpr]) -> Result<Expr> {
    Ok(match operands {
        [ParseExpr::Symbol(name), expr] => Expr::VarDecl {
            name: name.to_string(),
            value: Box::new(analyze_expr(expr)?),
        },
        _ => bail!("variable declarations must have two arguments, the first being a symbol"),
    })
}

fn analyze_assign(operands: &[ParseExpr]) -> Result<Expr> {
    Ok(match operands {
        [ParseExpr::Symbol(name), expr] => Expr::VarAssign {
            name: name.to_string(),
            value: Box::new(analyze_expr(expr)?),
        },
        _ => bail!("variable declarations must have two arguments, the first being a symbol"),
    })
}

fn analyze_loop(operands: &[ParseExpr]) -> Result<Expr> {
    Ok(Expr::Loop(Box::new(analyze(operands)?)))
}

fn analyze_break(operands: &[ParseExpr]) -> Result<Expr> {
    if operands.is_empty() {
        Ok(Expr::Break)
    } else {
        bail!("break expressions take no arguments")
    }
}

fn analyze_continue(operands: &[ParseExpr]) -> Result<Expr> {
    if operands.is_empty() {
        Ok(Expr::Continue)
    } else {
        bail!("continue expressions take no arguments")
    }
}

fn analyze_return(operands: &[ParseExpr]) -> Result<Expr> {
    Ok(match operands {
        [] => Expr::Return(None),
        [expr] => Expr::Return(Some(Box::new(analyze_expr(expr)?))),
        _ => bail!("return statements have one optional argument"),
    })
}

fn analyze_input(operands: &[ParseExpr]) -> Result<Expr> {
    if operands.is_empty() {
        Ok(Expr::Input)
    } else {
        bail!("input expressions take no arguments")
    }
}

fn analyze_expr(expr: &ParseExpr) -> Result<Expr> {
    Ok(match expr {
        ParseExpr::Integer(val) => Expr::IntegerLiteral(*val),
        ParseExpr::List(call_expr) => {
            if let Some((ParseExpr::Symbol(operator), operands)) = call_expr.split_first() {
                match operator.as_str() {
                    "+" => analyze_arithop(Operator::Add, operands)?,
                    "*" => analyze_arithop(Operator::Mul, operands)?,
                    "-" => analyze_arithop(Operator::Sub, operands)?,
                    "/" => analyze_arithop(Operator::Div, operands)?,
                    "if" => analyze_if(operands)?,
                    "define" => analyze_define(operands)?,
                    "set" => analyze_assign(operands)?,
                    "loop" => analyze_loop(operands)?,
                    "break" => analyze_break(operands)?,
                    "continue" => analyze_continue(operands)?,
                    "begin" => analyze(operands)?,
                    "return" => analyze_return(operands)?,
                    "input" => analyze_input(operands)?,
                    _ => bail!("invalid operator in call expression: {}", operator),
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
