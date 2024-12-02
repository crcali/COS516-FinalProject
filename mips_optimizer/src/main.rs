use egg::{define_language, rewrite as rw, *};
use std::fs;
use std::io::{self, Write};

// Define the RISC-V DSL with expanded instructions
define_language! {
    enum RiscvLang {
        // Arithmetic Instructions
        "add" = Add([Id; 3]),     // (add dest src1 src2)
        "sub" = Sub([Id; 3]),     // (sub dest src1 src2)
        "mul" = Mul([Id; 3]),     // (mul dest src1 src2)
        "div" = Div([Id; 3]),     // (div dest src1 src2)
        "rem" = Rem([Id; 3]),     // (rem dest src1 src2)
        "sll" = Sll([Id; 3]),     // (sll dest src shift)
        "srl" = Srl([Id; 3]),     // (srl dest src shift)
        "sra" = Sra([Id; 3]),     // (sra dest src shift)
        "and" = And([Id; 3]),     // (and dest src1 src2)
        "or" = Or([Id; 3]),       // (or dest src1 src2)
        "xor" = Xor([Id; 3]),     // (xor dest src1 src2)
        "slt" = Slt([Id; 3]),     // (slt dest src1 src2)
        "sltu" = Sltu([Id; 3]),   // (sltu dest src1 src2)

        // Immediate Instructions
        "addi" = Addi([Id; 3]),   // (addi dest src imm)
        "andi" = Andi([Id; 3]),   // (andi dest src imm)
        "ori" = Ori([Id; 3]),     // (ori dest src imm)
        "xori" = Xori([Id; 3]),   // (xori dest src imm)
        "slti" = Slti([Id; 3]),   // (slti dest src imm)
        "sltiu" = Sltiu([Id; 3]), // (sltiu dest src imm)
        "lui" = Lui([Id; 2]),     // (lui dest imm)
        "auipc" = Auipc([Id; 2]), // (auipc dest imm)

        // Branch Instructions
        "beq" = Beq([Id; 3]),     // (beq src1 src2 label)
        "bne" = Bne([Id; 3]),     // (bne src1 src2 label)
        "blt" = Blt([Id; 3]),     // (blt src1 src2 label)
        "bge" = Bge([Id; 3]),     // (bge src1 src2 label)
        "bltu" = Bltu([Id; 3]),   // (bltu src1 src2 label)
        "bgeu" = Bgeu([Id; 3]),   // (bgeu src1 src2 label)

        // Jump Instructions
        "jal" = Jal([Id; 2]),     // (jal dest label)
        "jalr" = Jalr([Id; 2]),   // (jalr dest src)

        // Load and Store Instructions
        "lw" = Lw([Id; 2]),       // (lw dest address)
        "sw" = Sw([Id; 2]),       // (sw src address)

        // Memory Addressing
        "mem" = Mem([Id; 2]),     // (mem base offset)

        // Pseudo-instructions
        "neg" = Neg([Id; 2]),     // (neg dest src)
        "not" = Not([Id; 2]),     // (not dest src)
        "inc" = Inc([Id; 2]),     // (inc dest src)
        "dec" = Dec([Id; 2]),     // (dec dest src)
        "move" = Move([Id; 2]),   // (move dest src)
        "nop" = Nop,              // (nop)

        // Unary Minus Operator
        "-" = NegOp([Id; 1]),     // Unary minus operator for negative numbers

        // Operands
        Var(Symbol),              // Registers or labels
        Num(i32),                 // Immediate values

        // For sequences of instructions
        "seq" = Seq(Vec<Id>),
    }
}

fn main() -> io::Result<()> {
    // Specify input and output file paths
    let input_file = "input.s";
    let output_file = "output.s";

    // Read the input file
    let input_content = fs::read_to_string(input_file)?;

    // Parse the input content into a RecExpr
    let expr: RecExpr<RiscvLang> = input_content.parse().unwrap();

    println!("Initial program:\n{}", expr);

    // Define rewrite rules for optimization
    let rules: &[Rewrite<RiscvLang, ()>] = &[
        // === Arithmetic Optimizations ===

        // Rule: Adding zero (add dest, src, 0) => addi dest, src, 0
        rw!("add-zero"; "(add ?dest ?src 0)" => "(addi ?dest ?src 0)"),

        // Rule: Subtracting zero (sub dest, src, 0) => addi dest, src, 0
        rw!("sub-zero"; "(sub ?dest ?src 0)" => "(addi ?dest ?src 0)"),

        // Rule: Multiplying by zero (mul dest, src, 0) => addi dest, zero, 0
        rw!("mul-zero"; "(mul ?dest ?src 0)" => "(addi ?dest zero 0)"),

        // Rule: Multiplying by one (mul dest, src, 1) => addi dest, src, 0
        rw!("mul-one"; "(mul ?dest ?src 1)" => "(addi ?dest ?src 0)"),

        // Rule: Dividing by one (div dest, src, 1) => addi dest, src, 0
        rw!("div-one"; "(div ?dest ?src 1)" => "(addi ?dest ?src 0)"),

        // Rule: Dividing zero by any number (div dest, 0, src) => addi dest, zero, 0
        rw!("div-zero"; "(div ?dest zero ?src)" => "(addi ?dest zero 0)"),

        // Rule: Subtracting from zero (sub dest, zero, src) => neg dest, src
        rw!("sub-zero-src"; "(sub ?dest zero ?src)" => "(neg ?dest ?src)"),

        // Rule: Negating a value (neg dest, src) => sub dest, zero, src
        rw!("neg-to-sub"; "(neg ?dest ?src)" => "(sub ?dest zero ?src)"),

        // Rule: Incrementing (addi dest, src, 1) => inc dest, src
        rw!("inc"; "(addi ?dest ?src 1)" => "(inc ?dest ?src)"),

        // Rule: Decrementing (addi dest, src, -1) => dec dest, src
        rw!("dec"; "(addi ?dest ?src -1)" => "(dec ?dest ?src)"),

        // Rule: Adding negative immediate (addi dest, src, (- ?imm)) => addi dest, src, -imm
        rw!("addi-neg-imm"; "(addi ?dest ?src (- ?imm))" => "(addi ?dest ?src (- ?imm))"),

        // Rule: Multiplying by 2 (mul dest, src, 2) => sll dest, src, 1
        rw!("mul-by-2"; "(mul ?dest ?src 2)" => "(sll ?dest ?src 1)"),

        // Rule: Dividing by 4 (div dest, src, 4) => sra dest, src, 2
        rw!("div-by-4"; "(div ?dest ?src 4)" => "(sra ?dest ?src 2)"),

        // === Logical Optimizations ===

        // Rule: ANDing with zero (and dest, src, 0) => addi dest, zero, 0
        rw!("and-zero"; "(and ?dest ?src 0)" => "(addi ?dest zero 0)"),

        // Rule: ORing with zero (or dest, src, 0) => addi dest, src, 0
        rw!("or-zero"; "(or ?dest ?src 0)" => "(addi ?dest ?src 0)"),

        // Rule: XORing with zero (xor dest, src, 0) => addi dest, src, 0
        rw!("xor-zero"; "(xor ?dest ?src 0)" => "(addi ?dest ?src 0)"),

        // Rule: XORing a value with itself (xor dest, src, src) => addi dest, zero, 0
        rw!("xor-same"; "(xor ?dest ?src ?src)" => "(addi ?dest zero 0)"),

        // Rule: NOT operation (not dest, src) => xori dest, src, -1
        rw!("not-to-xori"; "(not ?dest ?src)" => "(xori ?dest ?src -1)"),

        // Rule: Double NOT (not dest, (not _ ?src)) => addi dest, src, 0
        rw!("double-not"; "(not ?dest (not _ ?src))" => "(addi ?dest ?src 0)"),

        // === Shift Optimizations ===

        // Rule: Shift left by zero (sll dest, src, 0) => addi dest, src, 0
        rw!("sll-zero"; "(sll ?dest ?src 0)" => "(addi ?dest ?src 0)"),

        // Rule: Shift right logical by zero (srl dest, src, 0) => addi dest, src, 0
        rw!("srl-zero"; "(srl ?dest ?src 0)" => "(addi ?dest ?src 0)"),

        // Rule: Shift right arithmetic by zero (sra dest, src, 0) => addi dest, src, 0
        rw!("sra-zero"; "(sra ?dest ?src 0)" => "(addi ?dest ?src 0)"),

        // === Immediate Optimizations ===

        // Rule: Load immediate zero (addi dest, zero, 0) => addi dest, zero, 0
        rw!("load-zero"; "(addi ?dest zero 0)" => "(addi ?dest zero 0)"),

        // Rule: Load upper immediate zero (lui dest, 0) => addi dest, zero, 0
        rw!("lui-zero"; "(lui ?dest 0)" => "(addi ?dest zero 0)"),

        // === Branch Optimizations ===

        // Rule: Branch if equal to self (beq src, src, label) => jal zero, label
        rw!("beq-same"; "(beq ?src ?src ?label)" => "(jal zero ?label)"),

        // Rule: Branch if not equal to self (bne src, src, label) => nop
        rw!("bne-same"; "(bne ?src ?src ?label)" => "nop"),

        // === Data Transfer Optimizations ===

        // Rule: Load word with zero offset (lw dest, (add addr, 0)) => lw dest, addr
        rw!("lw-zero-offset"; "(lw ?dest (mem ?addr 0))" => "(lw ?dest ?addr)"),
        
        // Rule: Store word with zero offset (sw src, (add addr, 0)) => sw src, addr
        rw!("sw-zero-offset"; "(sw ?src (mem ?addr 0))" => "(sw ?src ?addr)"),

        // Rule: Load immediate to zero register (lw zero, _) => nop
        rw!("lw-zero-reg"; "(lw zero ?addr)" => "nop"),

        // Rule: Store from zero register (sw zero, _) => nop
        rw!("sw-zero-reg"; "(sw zero ?addr)" => "nop"),

        // === Miscellaneous Optimizations ===

        // Rule: Remove NOPs from sequences
        rw!("remove-nop"; "(seq ?instrs* nop ?rest*)" => "(seq ?instrs* ?rest*)"),

        // Rule: Remove redundant moves (addi dest, src, 0) where dest == src
        rw!("remove-redundant-move"; "(addi ?dest ?dest 0)" => "nop"),
    ];

    // Run the rewrite rules using the Runner
    let runner = Runner::default()
        .with_expr(&expr)
        .with_iter_limit(10)
        .run(rules);

    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);

    println!("\nOptimized program (cost {}):\n{}", best_cost, best_expr);

    let mut output = fs::File::create(output_file)?;
    write!(output, "{}", best_expr)?;

    println!("\nOptimized program written to {}", output_file);

    Ok(())
}
