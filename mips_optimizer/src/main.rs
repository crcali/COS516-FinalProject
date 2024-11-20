use egg::{define_language, rewrite as rw, *};
use std::fs;
use std::io::{self, Write};

define_language! {
    enum MipsLang {
        // Arithmetic Instructions
        "add" = Add([Id; 3]),     // (add dest src1 src2)
        "addi" = Addi([Id; 3]),   // (addi dest src imm)
        "sub" = Sub([Id; 3]),     // (sub dest src1 src2)
        "mul" = Mul([Id; 3]),     // (mul dest src1 src2)
        "div" = Div([Id; 3]),     // (div dest src1 src2)
        "slt" = Slt([Id; 3]),     // (slt dest src1 src2)
        "slti" = Slti([Id; 3]),   // (slti dest src imm)
        "and" = And([Id; 3]),     // (and dest src1 src2)
        "andi" = Andi([Id; 3]),   // (andi dest src imm)
        "or" = Or([Id; 3]),       // (or dest src1 src2)
        "ori" = Ori([Id; 3]),     // (ori dest src imm)
        "sll" = Sll([Id; 3]),     // (sll dest src shift)
        "srl" = Srl([Id; 3]),     // (srl dest src shift)
        "lui" = Lui([Id; 2]),     // (lui dest imm)
        "mov" = Mov([Id; 2]),    // (mov dest src)
        "mfhi" = Mfhi(Id),        // (mfhi dest)
        "mflo" = Mflo(Id),        // (mflo dest)

        // Data Transfer Instructions
        "lw" = Lw([Id; 2]),       // (lw dest addr)
        "sw" = Sw([Id; 2]),       // (sw src addr)
        "li" = Li([Id; 2]),       // (li dest imm)

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
        rw!("add-zero"; "(add ?dest ?src zero)" => "(mov ?dest ?src)"),
        rw!("add-zero-comm"; "(add ?dest zero ?src)" => "(mov ?dest ?src)"),
        rw!("addi-zero"; "(addi ?dest ?src 0)" => "(mov ?dest ?src)"),
        rw!("sub-zero"; "(sub ?dest ?src zero)" => "(mov ?dest ?src)"),
        rw!("mul-two"; "(mul ?dest ?src 2)" => "(sll ?dest ?src 1)"),
        rw!("sll-zero"; "(sll ?dest ?src 0)" => "(mov ?dest ?src)"),
        rw!("mov-to-self"; "(mov ?dest ?dest)" => "nop"),
        rw!("addi-zero-src"; "(addi ?dest zero ?imm)" => "(li ?dest ?imm)"),
        rw!("and-minus-one"; "(and ?dest ?src -1)" => "(mov ?dest ?src)"),
        rw!("or-zero"; "(or ?dest ?src zero)" => "(mov ?dest ?src)"),
        rw!("andi-zero"; "(andi ?dest ?src 0)" => "(li ?dest 0)"),
        rw!("slt-same"; "(slt ?dest ?src ?src)" => "(li ?dest 0)"),
    ];

    // Run the rewrite rules using the Runner
    let runner = Runner::default().with_expr(&expr).run(rules);

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
