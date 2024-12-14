use egg::{define_language, rewrite as rw, *};
use std::fs;
use std::io::{self, Write};

fn remove_nops(input: &str) -> String {
    let mut cleaned_lines = Vec::new();
    let mut seq_buffer = String::new();
    let mut in_seq = false;
    let mut open_parens_count = 0;

    for line in input.lines() {
        let trimmed = line.trim();

        if trimmed == "nop" {
            continue;
        }

        if trimmed.starts_with("(seq") {
            in_seq = true;
            seq_buffer.push_str(trimmed);
            seq_buffer.push(' ');
            open_parens_count += trimmed.matches('(').count();
            open_parens_count -= trimmed.matches(')').count();
        } else if in_seq {
            seq_buffer.push_str(trimmed);
            seq_buffer.push(' ');
            open_parens_count += trimmed.matches('(').count();
            open_parens_count -= trimmed.matches(')').count();

            if open_parens_count == 0 {
                let cleaned_sequence = clean_sequence(&seq_buffer);
                cleaned_lines.push(cleaned_sequence);
                seq_buffer.clear();
                in_seq = false;
            }
        } else {
            cleaned_lines.push(trimmed.to_string());
        }
    }

    if in_seq {
        let cleaned_sequence = clean_sequence(&seq_buffer);
        cleaned_lines.push(cleaned_sequence);
    }

    cleaned_lines.join("\n")
}

fn clean_sequence(seq: &str) -> String {
    let s = seq.trim();
    assert!(s.starts_with("(seq"));
    assert!(s.ends_with(')'));

    let inner = &s["(seq".len()..]; 
    let inner = inner.trim_start();

    let inner = &inner[..inner.len()-1];

    let cleaned = inner
        .split_whitespace()
        .filter(|&instr| instr != "nop")
        .collect::<Vec<&str>>()
        .join(" ");

    format!("(seq {})", cleaned)
}


define_language! {
    enum RiscvLang {
        // R-type instructions (register-register)
        "add" = Add([Id; 3]),
        "sub" = Sub([Id; 3]),
        "mul" = Mul([Id; 3]),
        "div" = Div([Id; 3]),
        "divu" = Divu([Id; 3]),
        "rem" = Rem([Id; 3]),
        "remu" = Remu([Id; 3]),
        "sll" = Sll([Id; 3]),
        "srl" = Srl([Id; 3]),
        "sra" = Sra([Id; 3]),
        "and" = And([Id; 3]),
        "or" = Or([Id; 3]),
        "xor" = Xor([Id; 3]),
        "slt" = Slt([Id; 3]),
        "sltu" = Sltu([Id; 3]),

        // I-type instructions (register-immediate)
        "addi" = Addi([Id; 3]),
        "andi" = Andi([Id; 3]),
        "ori" = Ori([Id; 3]),
        "xori" = Xori([Id; 3]),
        "slti" = Slti([Id; 3]),
        "sltiu" = Sltiu([Id; 3]),
        "lui" = Lui([Id; 2]),
        "auipc" = Auipc([Id; 2]),
        "slli" = Slli([Id; 3]),
        "srli" = Srli([Id; 3]),
        "srai" = Srai([Id; 3]),

        // Branch and jump instructions
        "beq" = Beq([Id; 3]),
        "bne" = Bne([Id; 3]),
        "blt" = Blt([Id; 3]),
        "bge" = Bge([Id; 3]),
        "bltu" = Bltu([Id; 3]),
        "bgeu" = Bgeu([Id; 3]),
        "jal" = Jal([Id; 2]),
        "jalr" = Jalr([Id; 2]),

        // Load/Store
        "lw" = Lw([Id; 2]),
        "sw" = Sw([Id; 2]),

        // CSR write
        "csrw" = Csrw([Id; 2]),

        // Operands
        Var(Symbol),
        Num(i32),

        // Sequence and NOP
        "seq" = Seq(Vec<Id>),
        "nop" = Nop,
    }
}

fn main() -> io::Result<()> {
    let input_file = "input.s";
    let output_file = "output.s";

    let input_content = fs::read_to_string(input_file)?;
    let expr: RecExpr<RiscvLang> = input_content.parse().unwrap();

    println!("Initial program:\n{}", expr);

    let rules: &[Rewrite<RiscvLang, ()>] = &[
        // Add x0: (add dest src x0) => (addi dest src 0)
        rw!("add-zero"; "(add ?dest ?src x0)" => "(addi ?dest ?src 0)"),
        // Add x0 commute: (add dest x0 src) => (addi dest src 0)
        rw!("add-zero-commute"; "(add ?dest x0 ?src)" => "(addi ?dest ?src 0)"),
        // Sub x0: (sub dest src x0) => (addi dest src 0)
        rw!("sub-zero"; "(sub ?dest ?src x0)" => "(addi ?dest ?src 0)"),
        // Mul by x0: (mul dest src x0) => (addi dest x0 0)
        rw!("mul-zero"; "(mul ?dest ?src x0)" => "(addi ?dest x0 0)"),
        // Div zero numerator: (div dest x0 src) => (addi ?dest x0 0)
        rw!("div-zero"; "(div ?dest x0 ?src)" => "(addi ?dest x0 0)"),
        // Rem zero numerator: (rem dest x0 ?src) => (addi ?dest x0 0)
        rw!("rem-zero-num"; "(rem ?dest x0 ?src)" => "(addi ?dest x0 0)"),
        // And with x0: (and dest src x0) => (addi dest x0 0)
        rw!("and-zero"; "(and ?dest ?src x0)" => "(addi ?dest x0 0)"),
        // And with x0 commute: (and dest x0 src) => (addi ?dest x0 0)
        rw!("and-zero-commute"; "(and ?dest x0 ?src)" => "(addi ?dest x0 0)"),
        // Or with x0: (or dest src x0) => (addi dest src 0)
        rw!("or-zero"; "(or ?dest ?src x0)" => "(addi ?dest ?src 0)"),
        // Or with x0 commute: (or ?dest x0 ?src) => (addi ?dest ?src 0)
        rw!("or-zero-commute"; "(or ?dest x0 ?src)" => "(addi ?dest ?src 0)"),
        // XORing value with itself: (xor dest src src) => (addi dest x0 0)
        rw!("xor-same"; "(xor ?dest ?src ?src)" => "(addi ?dest x0 0)"),
        // SLT with same registers: (slt dest src src) = 0
        rw!("slt-same"; "(slt ?dest ?src ?src)" => "(addi ?dest x0 0)"),
        // SLTU with same registers: (sltu dest src src) = 0
        rw!("sltu-same"; "(sltu ?dest ?src ?src)" => "(addi ?dest x0 0)"),
        // Sub same: (sub dest src src) = 0
        rw!("sub-same"; "(sub ?dest ?src ?src)" => "(addi ?dest x0 0)"),

        // SLL by x0: (sll dest src x0) => (addi dest src 0)
        rw!("sll-zero"; "(sll ?dest ?src x0)" => "(addi ?dest ?src 0)"),
        // SRL by x0: (srl ?dest ?src x0) => (addi ?dest ?src 0)
        rw!("srl-zero"; "(srl ?dest ?src x0)" => "(addi ?dest ?src 0)"),
        // SRA by x0: (sra dest src x0) => (addi ?dest ?src 0)
        rw!("sra-zero"; "(sra ?dest ?src x0)" => "(addi ?dest ?src 0)"),

        // beq-same: (beq src src label) => (jal x0 label)
        rw!("beq-same"; "(beq ?src ?src ?label)" => "(jal x0 ?label)"),
        // bne-same: (bne src src label) => nop
        rw!("bne-same"; "(bne ?src ?src ?label)" => "nop"),
        // blt-same: (blt src src label) => nop (never taken)
        rw!("blt-same"; "(blt ?src ?src ?label)" => "nop"),
        // bge-same: (bge src src label) => (jal x0 ?label) (always taken)
        rw!("bge-same"; "(bge ?src ?src ?label)" => "(jal x0 ?label)"),
        // bltu-same: (bltu src src label) => nop
        rw!("bltu-same"; "(bltu ?src ?src ?label)" => "nop"),
        // bgeu-same: (bgeu src src label) => (jal x0 ?label)
        rw!("bgeu-same"; "(bgeu ?src ?src ?label)" => "(jal x0 ?label)"),

        // lw x0 ... = nop (no effect)
        rw!("lw-zero-reg"; "(lw x0 ?addr)" => "nop"),
        // sw x0 ... = nop (store x0 == store 0)
        rw!("sw-zero-reg"; "(sw x0 ?addr)" => "nop"),

        // Remove nop from sequences
        rw!("remove-nop"; "(seq ?instrs* nop ?rest*)" => "(seq ?instrs* ?rest*)"),

        // Remove (addi dest, dest, 0) -> nop
        rw!("addi-zero-self"; "(addi ?dest ?dest 0)" => "nop"),

        // Replace top-level empty sequences with `nop`
        rw!("replace-top-empty-seq"; "(seq)" => "nop"),

        // Remove instructions writing to x0 (R-type)
        rw!("add-zero-dest"; "(add x0 ?s1 ?s2)" => "nop"),
        rw!("sub-zero-dest"; "(sub x0 ?s1 ?s2)" => "nop"),
        rw!("mul-zero-dest"; "(mul x0 ?s1 ?s2)" => "nop"),
        rw!("div-zero-dest"; "(div x0 ?s1 ?s2)" => "nop"),
        rw!("divu-zero-dest"; "(divu x0 ?s1 ?s2)" => "nop"),
        rw!("rem-zero-dest"; "(rem x0 ?s1 ?s2)" => "nop"),
        rw!("remu-zero-dest"; "(remu x0 ?s1 ?s2)" => "nop"),
        rw!("sll-zero-dest"; "(sll x0 ?s1 ?s2)" => "nop"),
        rw!("srl-zero-dest"; "(srl x0 ?s1 ?s2)" => "nop"),
        rw!("sra-zero-dest"; "(sra x0 ?s1 ?s2)" => "nop"),
        rw!("and-zero-dest"; "(and x0 ?s1 ?s2)" => "nop"),
        rw!("or-zero-dest"; "(or x0 ?s1 ?s2)" => "nop"),
        rw!("xor-zero-dest"; "(xor x0 ?s1 ?s2)" => "nop"),
        rw!("slt-zero-dest"; "(slt x0 ?s1 ?s2)" => "nop"),
        rw!("sltu-zero-dest"; "(sltu x0 ?s1 ?s2)" => "nop"),

        // Remove instructions writing to x0 (I-type)
        rw!("addi-zero-dest"; "(addi x0 ?s ?i)" => "nop"),
        rw!("andi-zero-dest"; "(andi x0 ?s ?i)" => "nop"),
        rw!("ori-zero-dest"; "(ori x0 ?s ?i)" => "nop"),
        rw!("xori-zero-dest"; "(xori x0 ?s ?i)" => "nop"),
        rw!("slti-zero-dest"; "(slti x0 ?s ?i)" => "nop"),
        rw!("sltiu-zero-dest"; "(sltiu x0 ?s ?i)" => "nop"),
        rw!("lui-zero-dest"; "(lui x0 ?i)" => "nop"),
        rw!("auipc-zero-dest"; "(auipc x0 ?i)" => "nop"),
        rw!("slli-zero-dest"; "(slli x0 ?s ?i)" => "nop"),
        rw!("srli-zero-dest"; "(srli x0 ?s ?i)" => "nop"),

        // XOR with x0 = move
        rw!("xor-zero"; "(xor ?dest ?src x0)" => "(addi ?dest ?src 0)"),
        rw!("xor-zero-commute"; "(xor ?dest x0 ?src)" => "(addi ?dest ?src 0)"),

        // Merge consecutive addi instructions to the same dest
        //    e.g. (seq (addi x1 x2 5) (addi x1 x1 3) ...) => (seq (addi x1 x2 8) ...)
        rw!("fold-consecutive-addi";
            "(seq (addi ?dest ?src (Num ?c1)) (addi ?dest ?dest (Num ?c2)) ?rest*)"
            => "(seq (addi ?dest ?src (Num (+ ?c1 ?c2))) ?rest*)"
        ),

        // Fold sub-one: (sub ?dest ?src (Num 1)) => (addi ?dest ?src -1)
        rw!("fold-sub-one";
            "(sub ?dest ?src (Num 1))"
            => "(addi ?dest ?src -1)"
        ),

        // Cancel out consecutive addi + addi with negative immediate:
        //    (seq (addi ?d ?s c) (addi ?d ?d -c)) => (seq)
        rw!("fold-addi-then-subi";
            "(seq (addi ?dest ?src (Num ?c1)) (addi ?dest ?dest (Num ?c2)) ?rest*)"
            => "(seq ?rest*)"
        ),

        // Fold lw/sw of the same register+address in direct sequence => no-op
        //    (seq (lw ?r ?addr) (sw ?r ?addr) ...) => just remove them
        rw!("fold-lw-sw-same-address";
            "(seq (lw ?r ?addr) (sw ?r ?addr) ?rest*)"
            => "(seq ?rest*)"
        ),
    ];

    let runner = Runner::default()
        .with_expr(&expr)
        .with_iter_limit(100)
        .run(rules);

    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);

    let optimized_expr = remove_nops(&best_expr.to_string());

    println!("\nOptimized program (cost {}):\n{}", best_cost, optimized_expr);

    let mut output = fs::File::create(output_file)?;
    write!(output, "{}", optimized_expr)?;

    println!("\nOptimized program written to {}", output_file);

    Ok(())
}
