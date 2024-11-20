use egg::{define_language, rewrite as rw, *};
use std::fs;
use std::io::{self, Write};

// Define the MIPS DSL with expanded instructions
define_language! {
    enum MipsLang {
        // Arithmetic Instructions
        "add" = Add([Id; 3]),     // (add dest src1 src2)
        "addi" = Addi([Id; 3]),   // (addi dest src imm)
        "sub" = Sub([Id; 3]),     // (sub dest src1 src2)
        "subi" = Subi([Id; 3]),   // (subi dest src imm)
        "mul" = Mul([Id; 3]),     // (mul dest src1 src2)
        "div" = Div([Id; 3]),     // (div dest src1 src2)
        "slt" = Slt([Id; 3]),     // (slt dest src1 src2)
        "slti" = Slti([Id; 3]),   // (slti dest src imm)
        "and" = And([Id; 3]),     // (and dest src1 src2)
        "andi" = Andi([Id; 3]),   // (andi dest src imm)
        "or" = Or([Id; 3]),       // (or dest src1 src2)
        "ori" = Ori([Id; 3]),     // (ori dest src imm)
        "xor" = Xor([Id; 3]),     // (xor dest src1 src2)
        "xori" = Xori([Id; 3]),   // (xori dest src imm)
        "nor" = Nor([Id; 3]),     // (nor dest src1 src2)
        "sll" = Sll([Id; 3]),     // (sll dest src shift)
        "srl" = Srl([Id; 3]),     // (srl dest src shift)
        "sra" = Sra([Id; 3]),     // (sra dest src shift)
        "lui" = Lui([Id; 2]),     // (lui dest imm)
        "mov" = Mov([Id; 2]),     // (mov dest src)
        "mfhi" = Mfhi(Id),        // (mfhi dest)
        "mflo" = Mflo(Id),        // (mflo dest)
        "not" = Not([Id; 2]),     // (not dest src) - Added 'not' instruction

        // Data Transfer Instructions
        "lw" = Lw([Id; 2]),       // (lw dest addr)
        "sw" = Sw([Id; 2]),       // (sw src addr)
        "lb" = Lb([Id; 2]),       // (lb dest addr)
        "sb" = Sb([Id; 2]),       // (sb src addr)
        "li" = Li([Id; 2]),       // (li dest imm)
        "la" = La([Id; 2]),       // (la dest label)

        // Branch Instructions
        "beq" = Beq([Id; 3]),     // (beq src1 src2 label)
        "bne" = Bne([Id; 3]),     // (bne src1 src2 label)
        "bgt" = Bgt([Id; 3]),     // (bgt src1 src2 label)
        "bge" = Bge([Id; 3]),     // (bge src1 src2 label)
        "blt" = Blt([Id; 3]),     // (blt src1 src2 label)
        "ble" = Ble([Id; 3]),     // (ble src1 src2 label)
        "j" = Jmp(Id),            // (j label)
        "jr" = Jr(Id),            // (jr src)
        "jal" = Jal(Id),          // (jal label)

        // System Calls
        "syscall" = Syscall,

        // Operands
        Var(Symbol),              // Registers or labels
        "zero" = Zero,            // Zero register
        Num(i32),                 // Immediate values

        // For sequences of instructions
        "seq" = Seq(Vec<Id>),
        "nop" = Nop,
    }
}

fn main() -> io::Result<()> {
    // Specify input and output file paths
    let input_file = "input.s";
    let output_file = "output.s";

    // Read the input file
    let input_content = fs::read_to_string(input_file)?;

    // Parse the input content into a RecExpr
    let expr: RecExpr<MipsLang> = input_content.parse().unwrap();

    println!("Initial program:\n{}", expr);

    // Define rewrite rules for optimization
    let rules: &[Rewrite<MipsLang, ()>] = &[
        // Arithmetic optimizations

        // Rule: Adding zero (add dest, src, zero) => mov dest, src
        // If we add zero to a register, it's the same as moving the value
        rw!("add-zero"; "(add ?dest ?src zero)" => "(mov ?dest ?src)"),

        // Rule: Adding zero (add dest, zero, src) => mov dest, src
        // Commutative case
        rw!("add-zero-comm"; "(add ?dest zero ?src)" => "(mov ?dest ?src)"),

        // Rule: Subtracting zero (sub dest, src, zero) => mov dest, src
        rw!("sub-zero"; "(sub ?dest ?src zero)" => "(mov ?dest ?src)"),

        // Rule: Adding immediate zero (addi dest, src, 0) => mov dest, src
        rw!("addi-zero"; "(addi ?dest ?src 0)" => "(mov ?dest ?src)"),

        // Rule: Subtracting immediate zero (subi dest, src, 0) => mov dest, src
        rw!("subi-zero"; "(subi ?dest ?src 0)" => "(mov ?dest ?src)"),

        // Rule: Multiplying by 2 (mul dest, src, 2) => sll dest, src, 1
        // Strength reduction optimization
        rw!("mul-by-2"; "(mul ?dest ?src 2)" => "(sll ?dest ?src 1)"),

        // Rule: Dividing by 4 (div dest, src, 4) => srl dest, src, 2
        rw!("div-by-4"; "(div ?dest ?src 4)" => "(srl ?dest ?src 2)"),

        // Rule: Shift left by zero (sll dest, src, 0) => mov dest, src
        rw!("sll-zero"; "(sll ?dest ?src 0)" => "(mov ?dest ?src)"),

        // Rule: Move to self (mov dest, dest) => nop
        rw!("mov-to-self"; "(mov ?dest ?dest)" => "nop"),

        // Logical optimizations

        // Rule: ANDing with zero (and dest, src, zero) => li dest, 0
        // Any value AND zero is zero
        rw!("and-zero"; "(and ?dest ?src zero)" => "(li ?dest 0)"),

        // Rule: ANDing with -1 (and dest, src, -1) => mov dest, src
        // Any value AND -1 (0xFFFFFFFF) is the value itself
        rw!("and-minus-one"; "(and ?dest ?src -1)" => "(mov ?dest ?src)"),

        // Rule: ORing with zero (or dest, src, zero) => mov dest, src
        // Any value OR zero is the value itself
        rw!("or-zero"; "(or ?dest ?src zero)" => "(mov ?dest ?src)"),

        // Rule: XORing with zero (xor dest, src, zero) => mov dest, src
        rw!("xor-zero"; "(xor ?dest ?src zero)" => "(mov ?dest ?src)"),

        // Rule: XOR a value with itself (xor dest, src, src) => li dest, 0
        // Any value XOR itself is zero
        rw!("xor-same"; "(xor ?dest ?src ?src)" => "(li ?dest 0)"),

        // Rule: NOR a value with zero (nor dest, src, zero) => not dest, src
        // Implemented using pseudo-instruction not
        rw!("nor-zero"; "(nor ?dest ?src zero)" => "(not ?dest ?src)"),

        // Comparison optimizations

        // Rule: Set less than (slt dest, src, src) => li dest, 0
        // A value is not less than itself
        rw!("slt-same"; "(slt ?dest ?src ?src)" => "(li ?dest 0)"),

        // Data transfer optimizations

        // Rule: Load immediate zero (li dest, 0) => mov dest, zero
        rw!("li-zero"; "(li ?dest 0)" => "(mov ?dest zero)"),

        // Rule: Load upper immediate zero (lui dest, 0) => mov dest, zero
        rw!("lui-zero"; "(lui ?dest 0)" => "(mov ?dest zero)"),

        // Miscellaneous optimizations

        // Rule: Remove nop from sequences
        rw!("remove-nop"; "(seq ?instrs* nop ?rest*)" => "(seq ?instrs* ?rest*)"),

        // Rule: Remove redundant consecutive moves (mov dest, src followed by mov src, dest)
        rw!("remove-redundant-mov"; "(seq ?prefix* (mov ?dest ?src) (mov ?src ?dest) ?suffix*)" => "(seq ?prefix* (mov ?dest ?src) ?suffix*)"),
    ];

    // Run the rewrite rules using the Runner
    let runner = Runner::default()
        .with_expr(&expr)
        .with_iter_limit(10) // Allow sufficient iterations
        .run(rules);

    // Use an extractor with a cost function (e.g., AST size)
    let extractor = Extractor::new(&runner.egraph, AstSize);

    // Find the best (optimized) expression
    let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);

    println!("\nOptimized program (cost {}):\n{}", best_cost, best_expr);

    // Write the optimized program to the output file
    let mut output = fs::File::create(output_file)?;
    write!(output, "{}", best_expr)?;

    println!("\nOptimized program written to {}", output_file);

    Ok(())
}
