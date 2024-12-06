from z3 import *
import sys
import re

#---------------------------------------------------------------------------------------
# Instruction class 
#---------------------------------------------------------------------------------------
class Instruction:
    def __init__(self, opcode, operands, line_num):
        self.opcode = opcode
        self.operands = operands
        self.line_num = line_num

#---------------------------------------------------------------------------------------
# Parser for the RISC-V assembly code in seq format
#---------------------------------------------------------------------------------------
def parse_seq_riscv_code(filename):
    instructions = []
    with open(filename, 'r') as f:
        content = f.read()

    content = re.sub(r'\s+', ' ', content)

    seq_match = re.search(r'\(seq\s+(.*)\)', content)
    if seq_match:
        seq_content = seq_match.group(1)
    else:
        seq_content = content.strip()

    instr_patterns = re.findall(r'\(([^()]+)\)', seq_content)

    for idx, instr_str in enumerate(instr_patterns):
        tokens = instr_str.strip().split()
        if not tokens:
            continue
        
        if len(tokens) == 1 and tokens[0].endswith(':'):
            continue
        
        opcode = tokens[0]
        operands = [operand.rstrip(',') for operand in tokens[1:]]
        instructions.append(Instruction(opcode, operands, idx + 1))

    return instructions

#---------------------------------------------------------------------------------------
# Helper functions and global definitions
#---------------------------------------------------------------------------------------

# Given a dictionary of registers, state_versions, and a register name,
# allocate a new BitVec version of that register with incremented state version.
def new_reg_var(registers, state_versions, reg_name):
    state_versions[reg_name] += 1
    new_var = BitVec(f'{reg_name}#{state_versions[reg_name]}', 32)
    registers[reg_name] = new_var
    return new_var

# Given a register name, return its current Z3 variable.
# If the operand is an immediate (integer), return a BitVecVal.
def get_reg_var(registers, reg_name):
    if reg_name.lstrip('-').isdigit():
        return BitVecVal(int(reg_name), 32)
    return registers[reg_name]

#---------------------------------------------------------------------------------------
# Model a single program using SMT constructs
#
# Incorporates:
# - 32 general-purpose registers (x0 always zero)
# - Arithmetic, logical, shift, immediate operations using Z3
# - Memory as a symbolic array
# - Branches and Jumps with a symbolic PC
# - State-versioning for registers and memory
#
# Note: We assume no loops and a straight-line or at most one-branch scenario. 
#---------------------------------------------------------------------------------------
def model_program(instructions, prefix):
    registers = {f'x{i}': BitVec(f'{prefix}x{i}#0', 32) for i in range(32)}
    registers['x0'] = BitVecVal(0, 32)

    # Versioning for registers
    state_versions = {f'x{i}': 0 for i in range(32)}

    # Initialize memory as a symbolic array:
    # Memory: address -> value (32-bit)
    mem = Array(f'{prefix}mem#0', BitVecSort(32), BitVecSort(32))
    mem_version = 0

    # PC to track the instruction index (not just line number, here line_num is 
    # the index in instructions). We'll treat line_num as a positional index.
    # We'll represent PC as a BitVec and each step we create a new PC variable.
    # However, for simplicity, since instructions are linear, PC = line_num.
    # For branches, we do: PC_next = If(condition, target, PC+1)
    # This will create a chain of Ifs.
    
    # We'll keep track of the final PC after all instructions.
    # Each step we "execute" the instruction at PC. However, PC can be symbolic.
    # We'll handle symbolic PC by constructing a piecewise definition of final states.

    # Because branches introduce conditionals, we must build final states as expressions
    # that depend on the path. We'll accumulate constraints in a solver at a higher level.
    # Here, we just return final states as symbolic expressions.

    # We'll "execute" instructions in sequence. If a branch changes PC,
    # that will create conditional expressions.
    # For simplicity: we will assume no complex control flow (no infinite loops)
    # and just model PC updates symbolically.

    PC = BitVecVal(1, 32)

    for instr in instructions:
        opcode = instr.opcode
        operands = instr.operands

        registers['x0'] = BitVecVal(0, 32)

        def update_reg(dest, val):
            new_reg_var(registers, state_versions, dest)
            registers[dest] = val

        def update_mem(addr_expr, val_expr):
            nonlocal mem, mem_version
            mem_version += 1
            mem = Store(mem, addr_expr, val_expr)

        old_PC = PC
        next_PC = old_PC + 1

        # Arithmetic Instructions
        if opcode in ['add', 'sub', 'mul', 'div', 'rem']:
            dest, src1, src2 = operands
            src1_var = get_reg_var(registers, src1)
            src2_var = get_reg_var(registers, src2)
            if opcode == 'add':
                val = src1_var + src2_var
            elif opcode == 'sub':
                val = src1_var - src2_var
            elif opcode == 'mul':
                val = src1_var * src2_var
            elif opcode == 'div':
                val = If(src2_var != 0, src1_var / src2_var, BitVecVal(0, 32))
            elif opcode == 'rem':
                val = If(src2_var != 0, src1_var % src2_var, BitVecVal(0, 32))
            update_reg(dest, val)

        # Logical Instructions
        elif opcode in ['and', 'or', 'xor']:
            dest, src1, src2 = operands
            src1_var = get_reg_var(registers, src1)
            src2_var = get_reg_var(registers, src2)
            if opcode == 'and':
                val = src1_var & src2_var
            elif opcode == 'or':
                val = src1_var | src2_var
            elif opcode == 'xor':
                val = src1_var ^ src2_var
            update_reg(dest, val)

        # Shift Instructions
        elif opcode in ['sll', 'srl', 'sra']:
            dest, src, shamt = operands
            src_var = get_reg_var(registers, src)
            shamt_var = get_reg_var(registers, shamt)
            if opcode == 'sll':
                val = src_var << shamt_var
            elif opcode == 'srl':
                val = LShR(src_var, shamt_var)
            elif opcode == 'sra':
                val = src_var >> shamt_var
            update_reg(dest, val)

        # Immediate Instructions
        elif opcode in ['addi', 'andi', 'ori', 'xori', 'slti', 'sltiu']:
            dest, src, imm = operands
            src_var = get_reg_var(registers, src)
            imm_val = BitVecVal(int(imm), 32)
            if opcode == 'addi':
                val = src_var + imm_val
            elif opcode == 'andi':
                val = src_var & imm_val
            elif opcode == 'ori':
                val = src_var | imm_val
            elif opcode == 'xori':
                val = src_var ^ imm_val
            elif opcode == 'slti':
                val = If(src_var < imm_val, BitVecVal(1, 32), BitVecVal(0, 32))
            elif opcode == 'sltiu':
                val = If(ULT(src_var, imm_val), BitVecVal(1, 32), BitVecVal(0, 32))
            update_reg(dest, val)

        # Load Upper Immediate
        elif opcode == 'lui':
            dest, imm = operands
            imm_val = int(imm)
            val = BitVecVal(imm_val << 12, 32)
            update_reg(dest, val)

        # Branch Instructions: PC update becomes conditional
        elif opcode in ['beq', 'bne', 'blt', 'bge', 'bltu', 'bgeu']:
            src1, src2, label_str = operands
            src1_var = get_reg_var(registers, src1)
            src2_var = get_reg_var(registers, src2)
            target_line = int(label_str)

            if opcode == 'beq':
                condition = (src1_var == src2_var)
            elif opcode == 'bne':
                condition = (src1_var != src2_var)
            elif opcode == 'blt':
                condition = (src1_var < src2_var)
            elif opcode == 'bge':
                condition = (src1_var >= src2_var)
            elif opcode == 'bltu':
                condition = ULT(src1_var, src2_var)
            elif opcode == 'bgeu':
                condition = UGE(src1_var, src2_var)

            # Conditionally update PC
            next_PC = If(condition, BitVecVal(target_line, 32), old_PC + 1)

        # Jump Instructions
        elif opcode in ['jal', 'jalr']:
            if opcode == 'jal':
                dest, label_str = operands
                target_line = int(label_str)
                update_reg(dest, old_PC + 1)
                next_PC = BitVecVal(target_line, 32)
            elif opcode == 'jalr':
                # Simplified assumption: jalr dest, reg
                # Normally: jalr xN, offset(xM) sets PC = (reg + offset) & ~1
                # We simplify: assume label is in operands or immediate
                dest, base = operands
                base_val = get_reg_var(registers, base)
                update_reg(dest, old_PC + 1)
                # PC = base_val; we assume it's a direct line number for simplicity
                next_PC = base_val

        # Load and Store Instructions
        elif opcode in ['lw', 'sw']:
            dest_or_src, addr_operand = operands
            # Typically: lw xN offset(xM), means offset(xM)
            # We'll assume addr_operand is something like offset(xM). Let's parse that:
            # offset(xM) might appear as xM or an immediate followed by a reg.
            # If input format is simplified like "x1 x2" meaning lw x1, x2 (no offset),
            # we must handle that. For correctness, let's assume it's always offset(xM).
            # The problem: original instructions would look like (lw x1 0(x2))
            # If so, we must parse that. For now, let's assume that the second operand is
            # either a register (no offset) or imm(reg). We'll try to parse offset and base.
            addr_str = addr_operand
            base_reg = None
            offset_val = 0
            m = re.match(r'(\-?\d+)\((x\d+)\)', addr_str)
            if m:
                offset_val = int(m.group(1))
                base_reg = m.group(2)
            else:
                base_reg = addr_str

            base_val = get_reg_var(registers, base_reg)
            addr_val = base_val + BitVecVal(offset_val, 32)

            if opcode == 'lw':
                val = Select(mem, addr_val)
                update_reg(dest_or_src, val)
            else: # sw
                src_val = get_reg_var(registers, dest_or_src)
                update_mem(addr_val, src_val)

        registers['x0'] = BitVecVal(0, 32)

        PC = next_PC

    return registers, mem, PC

#---------------------------------------------------------------------------------------
# Verify equivalence of two programs
# If sat: Not equivalent (print counterexample)
# If unsat: Equivalent
#---------------------------------------------------------------------------------------
def verify_equivalence(file1, file2):
    instrs1 = parse_seq_riscv_code(file1)
    instrs2 = parse_seq_riscv_code(file2)

    # We'll model both programs.
    # For initial state, create symbolic initial registers and memory for both.
    # Actually, to find a counterexample, we must share the SAME initial state across both,
    # meaning the initial registers and memory arrays must be the same symbolic variables
    # for both programs, not separate.
    # Then run program 1 and program 2 transformations on these shared initial states
    # but we must clone them before since model_program expects a prefix.
    # Instead, we can do a trick: model_program will produce final states from a given prefix.
    # We'll create initial states and pass them in. We need a unified approach.

    # We will start with a set of "unprefixed" initial states:
    initial_registers = {f'x{i}': BitVec(f'init_x{i}', 32) for i in range(32)}
    initial_registers['x0'] = BitVecVal(0, 32)  # enforce x0=0

    # For memory, we also create a single initial memory array
    initial_mem = Array('init_mem', BitVecSort(32), BitVecSort(32))

    final_regs_1, final_mem_1, final_pc_1 = run_program_with_init(instrs1, initial_registers, initial_mem, 'p1_')
    final_regs_2, final_mem_2, final_pc_2 = run_program_with_init(instrs2, initial_registers, initial_mem, 'p2_')

    s = Solver()

    s.add(final_regs_1['x0'] == 0)
    s.add(final_regs_2['x0'] == 0)

    # We now check for a difference in final states.
    # We must ensure that for all registers and memory, the final states match.
    # We'll look for a counterexample, so we assert the negation of equivalence.
    # Equivalence means:
    # all registers match AND final memories match 

    reg_constraints = []
    for i in range(32):
        reg_name = f'x{i}'
        if reg_name != 'x0':
            reg_constraints.append(final_regs_1[reg_name] == final_regs_2[reg_name])

    reg_constraints.append(final_mem_1 == final_mem_2)

    reg_constraints.append(final_pc_1 == final_pc_2)

    s.add(Not(And(reg_constraints)))

    if s.check() == sat:
        print("Programs are not equivalent.")
        m = s.model()
        print("\nCounterexample:")
        for i in range(32):
            reg_name = f'x{i}'
            if reg_name == 'x0':
                continue
            val = m.eval(initial_registers[reg_name], model_completion=True)
            print(f"{reg_name} = {val}")
    else:
        print("Programs are equivalent.")


def run_program_with_init(instructions, init_registers, init_mem, prefix):
    registers = {}
    state_versions = {}
    for i in range(32):
        reg_name = f'x{i}'
        registers[reg_name] = init_registers[reg_name]
        state_versions[reg_name] = 0
    registers['x0'] = BitVecVal(0,32)

    mem = init_mem
    mem_version = 0

    PC = BitVecVal(1, 32)

    def update_reg(dest, val):
        new_reg_var(registers, state_versions, dest)
        registers[dest] = val

    def update_mem(addr_expr, val_expr):
        nonlocal mem, mem_version
        mem_version += 1
        mem = Store(mem, addr_expr, val_expr)

    for instr in instructions:
        opcode = instr.opcode
        operands = instr.operands
        old_PC = PC
        next_PC = old_PC + 1

        registers['x0'] = BitVecVal(0,32)

        # Arithmetic
        if opcode in ['add', 'sub', 'mul', 'div', 'rem']:
            dest, src1, src2 = operands
            src1_var = get_reg_var(registers, src1)
            src2_var = get_reg_var(registers, src2)
            if opcode == 'add':
                val = src1_var + src2_var
            elif opcode == 'sub':
                val = src1_var - src2_var
            elif opcode == 'mul':
                val = src1_var * src2_var
            elif opcode == 'div':
                val = If(src2_var != 0, src1_var / src2_var, BitVecVal(0, 32))
            elif opcode == 'rem':
                val = If(src2_var != 0, src1_var % src2_var, BitVecVal(0, 32))
            update_reg(dest, val)

        # Logical
        elif opcode in ['and', 'or', 'xor']:
            dest, src1, src2 = operands
            src1_var = get_reg_var(registers, src1)
            src2_var = get_reg_var(registers, src2)
            if opcode == 'and':
                val = src1_var & src2_var
            elif opcode == 'or':
                val = src1_var | src2_var
            elif opcode == 'xor':
                val = src1_var ^ src2_var
            update_reg(dest, val)

        # Shift
        elif opcode in ['sll', 'srl', 'sra']:
            dest, src, shamt = operands
            src_var = get_reg_var(registers, src)
            shamt_var = get_reg_var(registers, shamt)
            if opcode == 'sll':
                val = src_var << shamt_var
            elif opcode == 'srl':
                val = LShR(src_var, shamt_var)
            elif opcode == 'sra':
                val = src_var >> shamt_var
            update_reg(dest, val)

        # Immediate
        elif opcode in ['addi', 'andi', 'ori', 'xori', 'slti', 'sltiu']:
            dest, src, imm = operands
            src_var = get_reg_var(registers, src)
            imm_val = BitVecVal(int(imm), 32)
            if opcode == 'addi':
                val = src_var + imm_val
            elif opcode == 'andi':
                val = src_var & imm_val
            elif opcode == 'ori':
                val = src_var | imm_val
            elif opcode == 'xori':
                val = src_var ^ imm_val
            elif opcode == 'slti':
                val = If(src_var < imm_val, BitVecVal(1, 32), BitVecVal(0, 32))
            elif opcode == 'sltiu':
                val = If(ULT(src_var, imm_val), BitVecVal(1, 32), BitVecVal(0, 32))
            update_reg(dest, val)

        # LUI
        elif opcode == 'lui':
            dest, imm = operands
            imm_val = int(imm)
            val = BitVecVal(imm_val << 12, 32)
            update_reg(dest, val)

        # Branch
        elif opcode in ['beq', 'bne', 'blt', 'bge', 'bltu', 'bgeu']:
            src1, src2, label_str = operands
            if not label_str.lstrip('-').isdigit():
                continue
            src1_var = get_reg_var(registers, src1)
            src2_var = get_reg_var(registers, src2)
            target_line = int(label_str)
            if opcode == 'beq':
                condition = (src1_var == src2_var)
            elif opcode == 'bne':
                condition = (src1_var != src2_var)
            elif opcode == 'blt':
                condition = (src1_var < src2_var)
            elif opcode == 'bge':
                condition = (src1_var >= src2_var)
            elif opcode == 'bltu':
                condition = ULT(src1_var, src2_var)
            elif opcode == 'bgeu':
                condition = UGE(src1_var, src2_var)
            next_PC = If(condition, BitVecVal(target_line, 32), old_PC + 1)

        # Jump
        elif opcode in ['jal', 'jalr']:
            if opcode == 'jal':
                dest, label_str = operands
                if not label_str.lstrip('-').isdigit():
                    continue
                target_line = int(label_str)
                update_reg(dest, old_PC + 1)
                next_PC = BitVecVal(target_line, 32)
            else: # jalr
                dest, base = operands
                base_val = get_reg_var(registers, base)
                update_reg(dest, old_PC + 1)
                next_PC = base_val

        # Load/Store
        elif opcode in ['lw', 'sw']:
            dest_or_src, addr_operand = operands
            m = re.match(r'(\-?\d+)\((x\d+)\)', addr_operand)
            if m:
                offset_val = int(m.group(1))
                base_reg = m.group(2)
            else:
                offset_val = 0
                base_reg = addr_operand
            base_val = get_reg_var(registers, base_reg)
            addr_val = base_val + BitVecVal(offset_val, 32)

            if opcode == 'lw':
                val = Select(mem, addr_val)
                update_reg(dest_or_src, val)
            else: # sw
                src_val = get_reg_var(registers, dest_or_src)
                update_mem(addr_val, src_val)

        registers['x0'] = BitVecVal(0,32)

        PC = next_PC

    return registers, mem, PC


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: python riscv_verifier.py original.s optimized.s")
        sys.exit(1)

    file1 = sys.argv[1]
    file2 = sys.argv[2]

    verify_equivalence(file1, file2)
