use super::args::Arguments;
use std::io::Read;
use tree_sitter::Node;

struct State<'a> {
    arguments: Arguments,
    code: &'a [u8],
    col: usize,
    row: usize,
    level: usize,
    extra_indentation: usize,
}

impl State<'_> {
    fn indent(&mut self) {
        // println!("i: {}, e: {}", self.level, self.extra_indentation);
        for _ in 0..self.level {
            print!("    ");
        }
        for _ in 0..self.extra_indentation {
            print!(" ");
        }
        self.col += 4 * self.level + self.extra_indentation;
    }

    fn print(&mut self, string: &str) {
        print!("{}", string);
        self.col += string.len();
    }

    fn print_node(&mut self, node: Node) {
        self.print(node.utf8_text(self.code).unwrap());
    }

    fn println(&mut self, string: &str) {
        println!("{}", string);
        self.col = 0;
        self.row += 1;
    }

    fn maybe_set_extra_indentation(&mut self, value: usize) {
        if self.extra_indentation == 0 {
            self.extra_indentation = value;
        }
    }
}

pub fn beautify(arguments: Arguments) {
    let mut code: String = "".into();

    if let Some(file) = &arguments.file {
        code = std::fs::read_to_string(file).expect("Could not read file.");
    } else {
        std::io::stdin()
            .read_to_string(&mut code)
            .expect("Error reading from stdin.");
    }

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_matlab::language())
        .expect("Error loading MATLAB grammar.");

    let tree = parser
        .parse(&code, None)
        .expect("An error occurred during parsing.");

    let root = tree.root_node();
    if root.has_error() {
        panic!("Error in tree, file is not valid.");
    }

    let mut state = State {
        arguments,
        code: code.as_bytes(),
        col: 0,
        row: 0,
        level: 0,
        extra_indentation: 0,
    };

    format_block(&mut state, root);
}

fn format_node(state: &mut State, node: Node) {
    match node.kind() {
        "assignment" => format_assignment(state, node),
        "binary_operator" => format_binary(state, node),
        "block" => format_block(state, node),
        "boolean_operator" => format_boolean(state, node),
        "cell" => format_matrix(state, node),
        "command" => format_command(state, node),
        "comment" => format_comment(state, node),
        "comparison_operator" => format_boolean(state, node),
        "field_expression" => format_field(state, node),
        "for_statement" => format_for(state, node),
        "function_call" => format_fncall(state, node),
        "global_operator" => format_global(state, node),
        "handle_operator" => format_unary(state, node),
        "if_statement" => format_if(state, node),
        "lambda" => format_lambda(state, node),
        "line_continuation" => format_line_continuation(state, node),
        "matrix" => format_matrix(state, node),
        "metaclass_operator" => format_unary(state, node),
        "multioutput_variable" => format_multioutput(state, node),
        "not_operator" => format_unary(state, node),
        "parenthesis" => format_parenthesis(state, node),
        "persistent_operator" => format_global(state, node),
        "postfix_operator" => format_unary(state, node),
        "range" => format_range(state, node),
        "row" => format_row(state, node),
        "switch_statement" => format_switch(state, node),
        "try_statement" => format_try(state, node),
        "unary_operator" => format_unary(state, node),
        "while_statement" => format_while(state, node),
        _ => state.print_node(node),
    }
}

fn format_block(state: &mut State, node: Node) {
    let statements = vec![
        "comment",
        "for_statement",
        "if_statement",
        "switch_statement",
        "try_statement",
        "while_statement",
    ];
    let mut cursor = node.walk();
    state.extra_indentation = 0;
    state.indent();
    let named_children: Vec<Node> = node.named_children(&mut cursor).collect();
    for (i, child) in named_children.iter().enumerate() {
        let previous = if i > 0 {
            named_children.get(i - 1)
        } else {
            None
        };
        let next = named_children.get(i + 1);
        if let Some(previous) = previous {
            // There are some empty lines between nodes. Preserve one of them.
            if child.range().start_point.row - previous.range().end_point.row > 1 {
                state.println("");
            }
            // Only assignments are allowed on the same line.
            if child.kind() != "assignment" || previous.kind() != "assignment" {
                state.println("");
                state.indent();
            }
        }
        format_node(state, *child);
        state.extra_indentation = 0;
        // Some statements don't have ; at the end, like if, for, while, etc.
        if !statements.contains(&child.kind()) {
            if let Some(next) = next {
                // If the current and next nodes are both assignments and on the same line, then
                // separate with , instead of ;
                if child.kind() == "assignment"
                    && next.kind() == "assignment"
                    && child.range().end_point.row == next.range().start_point.row
                {
                    state.print(", ");
                } else {
                    state.print(";");
                }
            } else {
                state.print(";");
            }
        }
    }
    state.extra_indentation = 0;
    state.println("");
}

fn format_comment(state: &mut State, node: Node) {
    let text = node.utf8_text(state.code).unwrap();
    if node.range().start_point.row != node.range().end_point.row {
        if text.starts_with("%{") {
            let lines: Vec<&str> = text
                .strip_prefix("%{")
                .unwrap()
                .strip_suffix("%}")
                .unwrap()
                .split('\n')
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            state.println("%{");
            state.extra_indentation = 2;
            for line in lines {
                state.indent();
                state.println(line);
            }
            state.extra_indentation = 0;
            state.indent();
            state.print("%}");
        } else {
            let lines: Vec<&str> = text
                .split('\n')
                .map(|l| l.strip_prefix('%').unwrap().trim())
                .collect();
            let mut first = true;
            for line in lines {
                if !first {
                    state.println("");
                    state.indent();
                }
                state.print("% ");
                state.print(line);
                first = false;
            }
        }
    } else {
        if state.col == state.level * 4 {
            if text.starts_with("%#") {
                state.print("%");
            } else {
                state.print("% ");
            }
        } else if text.starts_with("%#") {
            state.print(" %");
        } else {
            state.print(" % ");
        }
        state.print(text.strip_prefix('%').unwrap().trim());
    }
}

fn format_line_continuation(state: &mut State, node: Node) {
    state.print(" ");
    state.print_node(node);
    if node.range().start_point.row == node.range().end_point.row {
        state.println("");
    } else {
        state.col = 0;
        state.row += 1;
    }
    state.indent();
}

fn format_assignment(state: &mut State, node: Node) {
    let lhs = node.child_by_field_name("left").unwrap();
    let rhs = node.child_by_field_name("right").unwrap();
    format_node(state, lhs);
    state.print(" = ");
    format_node(state, rhs);
    state.extra_indentation = 0;
}

fn format_binary(state: &mut State, node: Node) {
    state.maybe_set_extra_indentation(state.col - 4 * state.level);
    let mut line_cont = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            line_cont = child.kind() == "line_continuation";
            format_node(state, child);
        } else {
            let operator = child.utf8_text(state.code).unwrap().trim();
            if state.arguments.sparse_math {
                if !line_cont {
                    state.print(" ");
                }
                state.maybe_set_extra_indentation(state.col - 4 * state.level);
                state.print(operator);
                state.print(" ");
            } else {
                state.maybe_set_extra_indentation(state.col - 4 * state.level);
                state.print(operator);
            }
        }
    }
}

fn format_boolean(state: &mut State, node: Node) {
    state.maybe_set_extra_indentation(state.col - 4 * state.level);
    let mut line_cont = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            line_cont = child.kind() == "line_continuation";
            format_node(state, child);
        } else {
            let operator = child.utf8_text(state.code).unwrap().trim();
            if !line_cont {
                state.print(" ");
            }
            state.maybe_set_extra_indentation(state.col - 4 * state.level);
            state.print(operator);
            state.print(" ");
        }
    }
}

fn format_unary(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let children = node
        .children(&mut cursor)
        .filter(|f| f.kind() != "line_continuation");
    for child in children {
        format_node(state, child);
    }
}

fn format_parenthesis(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let child = node
        .named_children(&mut cursor)
        .find(|c| c.kind() != "line_continuation")
        .unwrap();
    state.print("(");
    state.maybe_set_extra_indentation(state.col - 4 * state.level);
    format_node(state, child);
    state.print(")");
}

fn format_range(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let children = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "line_continuation");
    let mut first = true;
    let sparse = state.arguments.sparse_math;
    state.arguments.sparse_math = false;
    for child in children {
        if !first {
            state.print(":");
        }
        format_node(state, child);
        first = false;
    }
    state.arguments.sparse_math = sparse;
}

fn format_multioutput(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let children = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "line_continuation");
    state.print("[");
    let mut first = true;
    for child in children {
        if !first {
            state.print(",");
        }
        format_node(state, child);
        first = false;
    }
    state.print("]");
}

fn format_lambda(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let arguments = node.children(&mut cursor).find(|c| c.kind() == "arguments");
    let expression = node.child_by_field_name("expression").unwrap();
    state.print("@");
    state.print("(");
    if let Some(args) = arguments {
        let children = args
            .named_children(&mut cursor)
            .filter(|c| c.kind() != "line_continuation");
        let mut first = true;
        for arg in children {
            if !first {
                state.print(", ");
            }
            state.print_node(arg);
            first = false;
        }
    }
    state.print(") ");
    format_node(state, expression);
}

fn format_fncall(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let children = node.children(&mut cursor);
    let mut parens = true;
    for child in children {
        if child.kind() == "line_continuation" {
            continue;
        }
        if !child.is_named() {
            if child.utf8_text(state.code).unwrap() == "(" {
                break;
            } else if child.utf8_text(state.code).unwrap() == "{" {
                parens = false;
                break;
            }
        }
        format_node(state, child);
    }
    if parens {
        state.print("(");
    } else {
        state.print("{");
    }
    let prev_extra = state.extra_indentation;
    state.extra_indentation = state.col - 4 * state.level;
    let arguments = node.children(&mut cursor).find(|c| c.kind() == "arguments");
    if let Some(args) = arguments {
        format_arguments(state, args);
    }
    if parens {
        state.print(")");
    } else {
        state.print("}");
    }
    state.extra_indentation = prev_extra;
}

fn format_arguments(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let mut first = true;
    for child in node.named_children(&mut cursor) {
        if !first {
            state.print(", ");
        }
        format_node(state, child);
        first = false;
    }
}

fn format_command(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let mut first = true;
    for child in node.children(&mut cursor) {
        if !first {
            state.print(" ");
        }
        format_node(state, child);
        if child.kind() == "command_name" {
            state.extra_indentation = state.col - 4 * state.level;
        }
        first = false;
    }
    state.extra_indentation = 0;
}

fn format_field(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let mut first = true;
    let children = node.named_children(&mut cursor).filter(|c| !c.is_extra());
    for child in children {
        if !first {
            state.print(".");
        }
        format_node(state, child);
        first = false;
    }
}

fn format_matrix(state: &mut State, node: Node) {
    let matrix = node.kind() == "matrix";
    let multiline = node.range().start_point.row != node.range().end_point.row;
    let mut cursor = node.walk();
    if matrix {
        state.print("[");
    } else {
        state.print("{");
    }
    let prev_extra = state.extra_indentation;
    state.extra_indentation = state.col - 4 * state.level;
    let mut first = true;
    for child in node.named_children(&mut cursor) {
        if child.kind() == "comment" {
            if !first {
                state.print(";");
            }
            format_comment(state, child);
            state.println("");
            state.indent();
            first = true;
            continue;
        }
        if !first {
            if multiline {
                state.println(";");
                state.indent();
            } else {
                state.print("; ");
            }
        }
        format_node(state, child);
        if !child.is_extra() {
            first = false;
        }
    }
    if matrix {
        state.print("]");
    } else {
        state.print("}");
    }
    state.extra_indentation = prev_extra;
}

fn format_row(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let mut first = true;
    for child in node.named_children(&mut cursor) {
        if !first && !child.is_extra() {
            state.print(" ");
        }
        format_node(state, child);
        first = child.is_extra();
    }
}

fn format_global(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let mut first = true;
    let children = node
        .children(&mut cursor)
        .filter(|c| c.kind() != "line_continuation");
    for child in children {
        if !first {
            state.print(" ");
        }
        state.print_node(child);
        first = false;
    }
}

fn format_while(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let condition = node.child_by_field_name("condition").unwrap();
    let body = node
        .children(&mut cursor)
        .find(|c| c.kind() == "block")
        .unwrap();
    state.print("while ");
    format_node(state, condition);
    state.println("");
    state.level += 1;
    format_block(state, body);
    state.level -= 1;
    state.indent();
    state.print("end");
}

fn format_try(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let body = node
        .children(&mut cursor)
        .find(|c| c.kind() == "block")
        .unwrap();
    let catch = node
        .children(&mut cursor)
        .find(|c| c.kind() == "catch_clause")
        .unwrap();
    let catch_capture = catch
        .children(&mut cursor)
        .find(|c| c.kind() == "identifier");
    let catch_body = catch
        .children(&mut cursor)
        .find(|c| c.kind() == "block")
        .unwrap();
    state.println("try");
    state.level += 1;
    format_block(state, body);
    state.level -= 1;
    state.indent();
    state.print("catch");
    if let Some(capture) = catch_capture {
        state.print(" ");
        state.print_node(capture);
    }
    state.println("");
    state.level += 1;
    format_block(state, catch_body);
    state.level -= 1;
    state.indent();
    state.print("end");
}

fn format_switch(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let condition = node.child_by_field_name("condition").unwrap();
    let cases: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "case_clause")
        .collect();
    state.print("switch ");
    format_node(state, condition);
    state.println("");
    state.level += 1;
    for case in cases {
        let condition = case.child_by_field_name("condition").unwrap();
        let block = case
            .children(&mut cursor)
            .find(|c| c.kind() == "block")
            .unwrap();
        state.indent();
        state.print("case ");
        format_node(state, condition);
        state.println("");
        state.level += 1;
        format_block(state, block);
        state.level -= 1;
    }
    let otherwise = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "otherwise_clause");
    if let Some(otherwise) = otherwise {
        let block = otherwise
            .children(&mut cursor)
            .find(|c| c.kind() == "block")
            .unwrap();
        state.indent();
        state.println("otherwise");
        state.level += 1;
        format_block(state, block);
        state.level -= 1;
    }
    state.level -= 1;
    state.indent();
    state.print("end");
}

fn format_if(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let condition = node.child_by_field_name("condition").unwrap();
    let block = node
        .children(&mut cursor)
        .find(|c| c.kind() == "block")
        .unwrap();
    let elseif_clauses: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "elseif_clause")
        .collect();
    let else_clause = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "else_clause");
    state.print("if ");
    format_node(state, condition);
    state.println("");
    state.level += 1;
    format_block(state, block);
    state.level -= 1;
    for clause in elseif_clauses {
        let condition = clause.child_by_field_name("condition").unwrap();
        let block = clause
            .children(&mut cursor)
            .find(|c| c.kind() == "block")
            .unwrap();
        state.print("elseif ");
        format_node(state, condition);
        state.println("");
        state.level += 1;
        format_block(state, block);
        state.level -= 1;
    }
    if let Some(else_clause) = else_clause {
        let block = else_clause
            .children(&mut cursor)
            .find(|c| c.kind() == "block")
            .unwrap();
        state.println("else");
        state.level += 1;
        format_block(state, block);
        state.level -= 1;
    }
    state.indent();
    state.print("end");
}

fn format_for(state: &mut State, node: Node) {
    let mut cursor = node.walk();
    let parfor = node.child(0).unwrap().utf8_text(state.code).unwrap();
    state.print(parfor);
    state.print(" ");
    let iterator = node
        .children(&mut cursor)
        .find(|c| c.kind() == "iterator")
        .unwrap();
    let block = node
        .children(&mut cursor)
        .find(|c| c.kind() == "block")
        .unwrap();
    let parfor_options = node
        .children(&mut cursor)
        .find(|c| c.kind() == "parfor_options");
    if let Some(options) = parfor_options {
        state.print("(");
        state.print_node(iterator.named_child(0).unwrap());
        state.print(" = ");
        format_node(state, iterator.named_child(1).unwrap());
        state.print(", ");
        state.print_node(options.named_child(0).unwrap());
        state.print(")");
    } else {
        state.print_node(iterator.named_child(0).unwrap());
        state.print(" = ");
        format_node(state, iterator.named_child(1).unwrap());
    }
    state.println("");
    state.level += 1;
    format_block(state, block);
    state.level -= 1;
    state.indent();
    state.print("end");
}
