use anyhow::Result;

use self::core_structs::Function;
use self::gen::{gen_expr, Frame};
use crate::semantics::Expr;

mod core_structs;
mod gen;
mod instructions;

pub fn gen_ssa(expr: &mut Expr) -> Result<Function> {
    let mut func = Function::new();
    let mut frame = Frame::new();
    let start_block = func.start_block.clone();
    gen_expr(expr, &mut func, &mut frame, &mut vec![], start_block)?;
    // TODO: actually bring it into SSA form!
    Ok(func)
}
