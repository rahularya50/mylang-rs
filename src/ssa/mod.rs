use anyhow::Result;

use self::core_structs::Function;
use self::gen::{gen_expr_ssa, Frame};
use crate::semantics::Expr;

mod core_structs;
mod gen;
mod instructions;

pub fn gen_ssa(expr: &Expr) -> Result<Function> {
    let mut func = Function::new();
    let mut frame = Frame::new();
    let start_block = func.start_block.clone();
    gen_expr_ssa(expr, &mut func, &mut frame, start_block)?;
    Ok(func)
}
