# mylang

To compile a sample file, run `cargo run -- --target program.lang`.

## Language Semantics
Essentially, the language supports:
- Variable definition as `(define <name> <value>)`
- Variable reassignment as `(set <name> <value>)`
- If statements (comparing the predicate to `0`) as `(if <pred> <conseq> [alt])`
- Unconditional loops, as `(loop <body1> ...)`
- Break and continue statements as `(break)` and `(continue)`
- Arithmetic operators
- Function definition and returns using `(func (<name> <arg1> ...) <expr1> ...)` and `(return [expr])`

The only data type supported is the integer.

See https://github.com/rahularya50/mylang-rs/blob/master/src/semantics/mod.rs for exact details.

All expressions can potentially evaluate to a value. 
Details on evaluation are in https://github.com/rahularya50/mylang-rs/blob/master/src/ir/gen.rs.
At a high level, variable declaration evaluates to the initialized value and 
if statements evaluate to a value iff the consequent and alternate both do.

## Compiler Frontned
1. A straightforward lexer and parser take the input file and convert it into a tree of `ParseExpr`s, in `src/frontend/`.
2. Semantic analysis generates a `Program` struct, viewed as a hierarchy of typed syntax elements, and verifies that all syntactic constructs above are used correctly, in `src/semantics/`
3. The `Program` struct is then lowered into a control-flow graph of basic blocks in `src/ir/gen.rs`, using a set of primitive instructions defined in `src/ir/instructions.rs`.

## SSA Generation
We use the dominance algorithm from Cooper et al. to compute the dominance frontier of each block:
1. We first sort all the basic blocks in the CFG by post-order.
2. Next, we identify the "immediate dominators" (`idoms`) of each basic block.
3. From the `idom[]` array, we compute the "dominance frontier" of each basic block, to see where variables defined in the block may encounter alternative control flow paths.

The implementation of these steps is in `src/ir/dominance.rs`.

Next, we generate phi blocks and renumber variables as `VirtualRegisters` (that can only be assigned to once) to avoid variable reassignment:

4. We identify which basic blocks define/redefine each variable in the function, producing a mapping `var_name -> List[Blocks]`
5. For each variable, starting at each of its defining blocks, we compute its "iterated dominance frontier" to see all the points where its conflicting definitions may need to be merged. 
   At each of these points, as well as at each of its defining blocks, we associate a new `VirtualRegister` with the variable name.  
6. We traverse the "dominator tree" from the root. At each block, we look up the latest `VirtualRegister` from the mapping generated previously for each variable still in scope,
   rename all references to that variable to use this new `VirtualRegister`. 
   If a new `VirtualRegister` was allocated for this block due to it lying on the iterated dominance frontier for a definition of each variable, we also insert a phi node for that `VirtualRegister` (with its ancestors left blank).
7. Finally, we traverse the entire dominator tree once more, looking for blocks whose children have phi nodes, and "backfill" their ancestors based on the `VirtualRegister` mapped from that register at each of the block's parents.

The implementation of these steps is in `src/ir/ssa_transform.rs`, and loosely based on the slides from https://groups.seas.harvard.edu/courses/cs252/2011sp/slides/Lec04-SSA.pdf (more references may be found in the source code).

## Optimization
Optimization passes may be found in `src/optimizations`. The main ones are:
- Block merging: If Block A jumps to Block B unconditionally, and there is no way to jump to Block B directly, we can merge the two blocks.
- Copy propagation: Assignments of the form `rx = ry` can be removed, with all references to `rx` replaced with `ry` (since each `VirtualRegister` is only assigned to once)
- Dead code elimination: Propagating backwards from `return` statements and control flow, we determine what registers are actually used either directly or indirectly, and delete all instructions involving unused registers.
- Constant folding: Using a lattice structure to model registers as being `Undefined`, a known constant, or `Variable`, we trace through the program and determine what registers are really just constants, and replace their assignment with constant-initialization.
 This optimization also handles control flow, by only taking branches that could potentially be taken at some point, in "aggressive constant folding".
- Loop-invariant code motion: TODO DOCS

## Compiler Backend
SSA form assumes we have an infinite number of registers. The backend determines register liveness by looking at definitions and consumers, and allocates physical registers for each `VirtualRegister`. 
If the number of live `VirtualRegisters` exeeds the number of available physical registers, we "spill" the least used `VirtualRegisters` onto the stack and load and store them only right as needed.

This is done as follows:

1. Liveness analysis processes the control flow graph backwards from consumers of a register and determines in which regions of each block is a register active. This takes place in `src/backend/register_liveness.rs`.
2. A "register interference graph" is built, with edges between registers iff their live regions overlap in some block.
3. Greedy coloring is done by sorting the vertices in the graph based on their "simplicial elimination ordering", and then allocating physical registers to each node in this order. 
   It turns out that, for interference graphs produced from SSA form, this algorithm guarantees that the minimum number of physical registers are used (even though coloring is NP-hard in general).
   See Section 6 of the lecture notes in https://www.cs.cmu.edu/~fp/courses/15411-f13/lectures/03-regalloc.pdf, or read the comments in the code for more details. The implementation is in `src/backend/register_coloring.rs`.
   
## Future work
Most of the remaining work lies in the code-generation phase of the compiler backend. Specifically, I still need to implement:

- RISC-V code generation
- Peephole analysis
- Instruction selection
