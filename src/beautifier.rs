/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::args::Arguments;
use anyhow::{anyhow, Context, Result};
use tree_sitter::Node;

struct State<'a> {
    formatted: String,
    arguments: &'a mut Arguments,
    code: &'a [u8],
    col: usize,
    row: usize,
    level: usize,
    extra_indentation: usize,
}

impl State<'_> {
    fn indent(&mut self) {
        for _ in 0..self.level {
            self.print("    ");
        }
        for _ in 0..self.extra_indentation {
            self.print(" ");
        }
    }

    fn print(&mut self, string: &str) {
        if self.arguments.inplace {
            self.formatted += string;
        } else {
            print!("{}", string);
        }
        self.col += string.len();
    }

    fn print_node(&mut self, node: Node) -> Result<()> {
        self.print(node.utf8_text(self.code)?);
        Ok(())
    }

    fn println(&mut self, string: &str) {
        if self.arguments.inplace {
            self.formatted += string;
            self.formatted += "\n";
        } else {
            println!("{}", string);
        }
        self.col = 0;
        self.row += 1;
    }

    fn maybe_set_extra_indentation(&mut self, value: usize) {
        if self.extra_indentation == 0 {
            self.extra_indentation = value;
        }
    }
}

trait TraversingError<T> {
    fn err_at_loc(self, node: &Node) -> Result<T>;
}

impl<T> TraversingError<T> for Option<T> {
    fn err_at_loc(self, node: &Node) -> Result<T> {
        self.ok_or_else(|| {
            anyhow!(
                "Error accessing token around line {} col {}",
                node.range().start_point.row,
                node.range().start_point.column
            )
        })
    }
}

pub fn beautify(code: &str, arguments: &mut Arguments) -> Result<String> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_matlab::language())
        .with_context(|| "Could not set Tree-Sitter language")?;

    let tree = parser
        .parse(code, None)
        .ok_or_else(|| anyhow!("Could not parse file."))?;

    let root = tree.root_node();
    if root.has_error() {
        return Err(anyhow!("Parsed file contain errors."));
    }

    let mut state = State {
        arguments,
        code: code.as_bytes(),
        col: 0,
        row: 0,
        level: 0,
        extra_indentation: 0,
        formatted: "".into(),
    };

    format_block(&mut state, root)?;
    Ok(state.formatted)
}

fn format_node(state: &mut State, node: Node) -> Result<()> {
    match node.kind() {
        "arguments_statement" => format_arguments_statement(state, node),
        "assignment" => format_assignment(state, node),
        "binary_operator" => format_binary(state, node),
        "block" => format_block(state, node),
        "boolean_operator" => format_boolean(state, node),
        "cell" => format_matrix(state, node),
        "class_definition" => format_classdef(state, node),
        "command" => format_command(state, node),
        "comment" => format_comment(state, node),
        "comparison_operator" => format_boolean(state, node),
        "field_expression" => format_field(state, node),
        "for_statement" => format_for(state, node),
        "function_call" => format_fncall(state, node),
        "function_definition" => format_function(state, node),
        "global_operator" => format_global(state, node),
        "handle_operator" => format_unary(state, node),
        "if_statement" => format_if(state, node),
        "indirect_access" => format_parenthesis(state, node),
        "lambda" => format_lambda(state, node),
        "line_continuation" => format_line_continuation(state, node),
        "matrix" => format_matrix(state, node),
        "metaclass_operator" => format_unary(state, node),
        "multioutput_variable" => format_multioutput(state, node),
        "not_operator" => format_unary(state, node),
        "parenthesis" => format_parenthesis(state, node),
        "persistent_operator" => format_global(state, node),
        "postfix_operator" => format_unary(state, node),
        "property" => format_property(state, node),
        "property_name" => format_property_name(state, node),
        "range" => format_range(state, node),
        "row" => format_row(state, node),
        "switch_statement" => format_switch(state, node),
        "try_statement" => format_try(state, node),
        "unary_operator" => format_unary(state, node),
        "while_statement" => format_while(state, node),
        _ => state.print_node(node),
    }
}

fn format_block(state: &mut State, node: Node) -> Result<()> {
    let statements = vec![
        "arguments_statement",
        "class_definition",
        "comment",
        "for_statement",
        "function_definition",
        "if_statement",
        "switch_statement",
        "try_statement",
        "while_statement",
    ];
    let mut cursor = node.walk();
    let original_indentation = state.level;
    let indents = vec!["cvx_begin", "subject"];
    let dedents = vec!["cvx_end"];
    state.extra_indentation = 0;
    state.indent();
    let mut named_children: Vec<Node> = node.named_children(&mut cursor).collect();
    let mut prev_node = node;
    while let Some(n) = prev_node.prev_named_sibling() {
        prev_node = n;
        if n.kind() == "comment" {
            if n.utf8_text(state.code)?.starts_with("%#") {
                continue;
            }
            named_children.insert(0, n);
        } else {
            break;
        }
    }
    let mut next_node = node;
    while let Some(n) = next_node.next_named_sibling() {
        next_node = n;
        if n.kind() == "comment" {
            named_children.push(n);
        } else {
            break;
        }
    }
    for (i, child) in named_children.iter().enumerate() {
        let previous = if i > 0 {
            named_children.get(i - 1)
        } else {
            None
        };
        let next = named_children.get(i + 1);
        if child.kind() == "command" {
            let command_name = child
                .named_child(0)
                .err_at_loc(child)?
                .utf8_text(state.code)?;
            if dedents.contains(&command_name) {
                state.level = original_indentation;
            }
        }
        if let Some(previous) = previous {
            // There are some empty lines between nodes. Preserve one of them.
            if child.range().start_point.row - previous.range().end_point.row > 1 {
                state.println("");
            }
            // Only assignments and comments are allowed on the same line.
            if !(child.kind() == "assignment" && previous.kind() == "assignment")
                && child.kind() != "comment"
                || child.range().start_point.row != previous.range().end_point.row
            {
                state.println("");
                state.indent();
            }
        }
        format_node(state, *child)?;
        state.extra_indentation = 0;
        if child.kind() == "command" {
            let command_name = child
                .named_child(0)
                .err_at_loc(child)?
                .utf8_text(state.code)?;
            if indents.contains(&command_name) {
                state.level += 1;
            }
        }
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
    state.level = original_indentation;
    state.println("");
    Ok(())
}

fn format_comment(state: &mut State, node: Node) -> Result<()> {
    let text = node.utf8_text(state.code).unwrap();
    if node.range().start_point.row != node.range().end_point.row {
        if text.starts_with("%{") {
            let lines: Vec<&str> = text
                .strip_prefix("%{")
                .unwrap_or(text)
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
                .map(|l| l.trim().strip_prefix('%').unwrap_or(l.trim()).trim())
                .collect();
            for (i, line) in lines.iter().enumerate() {
                let line = line.trim();
                if i != 0 {
                    state.println("");
                    state.indent();
                }
                state.print("%");
                if !line.is_empty() {
                    state.print(" ");
                }
                state.print(line);
            }
        }
    } else {
        let line = text.strip_prefix('%').unwrap_or(text).trim();
        if state.col == state.level * 4 {
            if text.starts_with("%#") || text.starts_with("%%") {
                state.print("%");
            } else {
                state.print("%");
                if !line.is_empty() {
                    state.print(" ");
                }
            }
        } else if text.starts_with("%#") || text.starts_with("%%") {
            state.print(" %");
        } else {
            state.print(" %");
            if !line.is_empty() {
                state.print(" ");
            }
        }
        state.print(line);
    }
    Ok(())
}

fn format_line_continuation(state: &mut State, node: Node) -> Result<()> {
    state.print(" ");
    state.print_node(node)?;
    if node.range().start_point.row == node.range().end_point.row {
        state.println("");
    } else {
        state.col = 0;
        state.row += 1;
    }
    state.indent();
    Ok(())
}

fn format_assignment(state: &mut State, node: Node) -> Result<()> {
    let lhs = node.child_by_field_name("left").err_at_loc(&node)?;
    let rhs = node.child_by_field_name("right").err_at_loc(&node)?;
    format_node(state, lhs)?;
    state.print(" = ");
    format_node(state, rhs)?;
    state.extra_indentation = 0;
    Ok(())
}

fn format_binary(state: &mut State, node: Node) -> Result<()> {
    state.maybe_set_extra_indentation(state.col - 4 * state.level);
    let add_ops = vec!["+", "-", ".+", ".-"];
    let mut line_cont = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            line_cont = child.kind() == "line_continuation";
            format_node(state, child)?;
        } else {
            let operator = child.utf8_text(state.code)?.trim();
            if state.arguments.sparse_math
                || state.arguments.sparse_add && add_ops.contains(&operator)
            {
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
    Ok(())
}

fn format_boolean(state: &mut State, node: Node) -> Result<()> {
    state.maybe_set_extra_indentation(state.col - 4 * state.level);
    let mut line_cont = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            line_cont = child.kind() == "line_continuation";
            format_node(state, child)?;
        } else {
            let operator = child.utf8_text(state.code)?.trim();
            if !line_cont {
                state.print(" ");
            }
            state.maybe_set_extra_indentation(state.col - 4 * state.level);
            state.print(operator);
            state.print(" ");
        }
    }
    Ok(())
}

fn format_unary(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children = node
        .children(&mut cursor)
        .filter(|f| f.kind() != "line_continuation");
    for child in children {
        format_node(state, child)?;
    }
    Ok(())
}

fn format_parenthesis(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let child = node
        .named_children(&mut cursor)
        .find(|c| c.kind() != "line_continuation")
        .err_at_loc(&node)?;
    state.print("(");
    state.maybe_set_extra_indentation(state.col - 4 * state.level);
    format_node(state, child)?;
    state.print(")");
    Ok(())
}

fn format_range(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "line_continuation");
    let sparse = state.arguments.sparse_math;
    state.arguments.sparse_math = false;
    for (i, child) in children.enumerate() {
        if i != 0 {
            state.print(":");
        }
        format_node(state, child)?;
    }
    state.arguments.sparse_math = sparse;
    Ok(())
}

fn format_multioutput(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() != "line_continuation");
    state.print("[");
    for (i, child) in children.enumerate() {
        if i != 0 {
            state.print(", ");
        }
        format_node(state, child)?;
    }
    state.print("]");
    Ok(())
}

fn format_lambda(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let arguments = node.children(&mut cursor).find(|c| c.kind() == "arguments");
    let expression = node.child_by_field_name("expression").err_at_loc(&node)?;
    state.print("@");
    state.print("(");
    if let Some(args) = arguments {
        let children = args
            .named_children(&mut cursor)
            .filter(|c| c.kind() != "line_continuation");
        for (i, arg) in children.enumerate() {
            if i != 0 {
                state.print(", ");
            }
            state.print_node(arg)?;
        }
    }
    state.print(") ");
    format_node(state, expression)?;
    Ok(())
}

fn format_fncall(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children = node.children(&mut cursor);
    let mut parens = true;
    for child in children {
        if child.kind() == "line_continuation" {
            continue;
        }
        if !child.is_named() {
            if child.utf8_text(state.code)? == "(" {
                break;
            } else if child.utf8_text(state.code)? == "{" {
                parens = false;
                break;
            }
        }
        format_node(state, child)?;
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
        format_arguments(state, args)?;
    }
    if parens {
        state.print(")");
    } else {
        state.print("}");
    }
    state.extra_indentation = prev_extra;
    Ok(())
}

fn format_arguments(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.named_children(&mut cursor).collect();
    for (i, child) in children.iter().enumerate() {
        if i != 0 && children.get(i - 1).unwrap().kind() != "line_continuation" {
            state.print(", ");
        }
        format_node(state, *child)?;
    }
    Ok(())
}

fn format_command(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    for (i, child) in node.children(&mut cursor).enumerate() {
        if i != 0 {
            state.print(" ");
        }
        format_node(state, child)?;
        if child.kind() == "command_name" {
            state.extra_indentation = state.col - 4 * state.level;
        }
    }
    state.extra_indentation = 0;
    Ok(())
}

fn format_field(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children = node.named_children(&mut cursor).filter(|c| !c.is_extra());
    for (i, child) in children.enumerate() {
        if i != 0 {
            state.print(".");
        }
        format_node(state, child)?;
    }
    Ok(())
}

fn format_matrix(state: &mut State, node: Node) -> Result<()> {
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
            format_comment(state, child)?;
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
        format_node(state, child)?;
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
    Ok(())
}

fn format_row(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let mut first = true;
    for child in node.named_children(&mut cursor) {
        if !first && !child.is_extra() {
            state.print(" ");
        }
        format_node(state, child)?;
        first = child.is_extra();
    }
    Ok(())
}

fn format_global(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let children = node
        .children(&mut cursor)
        .filter(|c| c.kind() != "line_continuation");
    for (i, child) in children.enumerate() {
        if i != 0 {
            state.print(" ");
        }
        state.print_node(child)?;
    }
    Ok(())
}

fn format_while(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let condition = node.child_by_field_name("condition").err_at_loc(&node)?;
    let body = node.children(&mut cursor).find(|c| c.kind() == "block");
    state.print("while ");
    format_node(state, condition)?;
    print_linter_comment(state, node)?;
    state.println("");
    state.level += 1;
    if let Some(body) = body {
        format_block(state, body)?;
    } else {
        print_non_linter_comments(state, node)?;
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_try(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let body = node.children(&mut cursor).find(|c| c.kind() == "block");
    let catch = node
        .children(&mut cursor)
        .find(|c| c.kind() == "catch_clause")
        .err_at_loc(&node)?;
    let catch_capture = catch
        .children(&mut cursor)
        .find(|c| c.kind() == "identifier");
    let catch_body = catch.children(&mut cursor).find(|c| c.kind() == "block");
    state.println("try");
    state.level += 1;
    if let Some(body) = body {
        format_block(state, body)?;
    } else {
        print_non_linter_comments(state, node)?;
    }
    state.level -= 1;
    state.indent();
    state.print("catch");
    if let Some(capture) = catch_capture {
        state.print(" ");
        state.print_node(capture)?;
    }
    print_linter_comment(state, catch)?;
    state.println("");
    state.level += 1;
    if let Some(catch_body) = catch_body {
        format_block(state, catch_body)?;
    } else {
        print_non_linter_comments(state, catch)?;
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_switch(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let condition = node.child_by_field_name("condition").err_at_loc(&node)?;
    let cases: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "case_clause")
        .collect();
    state.print("switch ");
    format_node(state, condition)?;
    print_linter_comment(state, node)?;
    state.println("");
    state.level += 1;
    for case in cases {
        let condition = case.child_by_field_name("condition").err_at_loc(&case)?;
        let block = case.children(&mut cursor).find(|c| c.kind() == "block");
        state.indent();
        state.print("case ");
        format_node(state, condition)?;
        print_linter_comment(state, case)?;
        state.println("");
        state.level += 1;
        if let Some(block) = block {
            format_block(state, block)?;
        } else {
            print_non_linter_comments(state, case)?;
        }
        state.level -= 1;
    }
    let otherwise = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "otherwise_clause");
    if let Some(otherwise) = otherwise {
        let block = otherwise
            .children(&mut cursor)
            .find(|c| c.kind() == "block");
        state.indent();
        state.println("otherwise");
        state.level += 1;
        if let Some(block) = block {
            format_block(state, block)?;
        } else {
            print_non_linter_comments(state, otherwise)?;
        }
        state.level -= 1;
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_if(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let condition = node.child_by_field_name("condition").err_at_loc(&node)?;
    let block = node.children(&mut cursor).find(|c| c.kind() == "block");
    let elseif_clauses: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "elseif_clause")
        .collect();
    let else_clause = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "else_clause");
    state.print("if ");
    format_node(state, condition)?;
    print_linter_comment(state, node)?;
    state.println("");
    state.level += 1;
    if let Some(block) = block {
        format_block(state, block)?;
    } else {
        print_non_linter_comments(state, node)?;
    }
    state.level -= 1;
    for clause in elseif_clauses {
        let condition = clause
            .child_by_field_name("condition")
            .err_at_loc(&clause)?;
        let block = clause.children(&mut cursor).find(|c| c.kind() == "block");
        state.indent();
        state.print("elseif ");
        format_node(state, condition)?;
        print_linter_comment(state, clause)?;
        state.println("");
        state.level += 1;
        state.extra_indentation = 0;
        if let Some(block) = block {
            format_block(state, block)?;
        } else {
            print_non_linter_comments_after(state, clause)?;
        }
        state.level -= 1;
    }
    if let Some(else_clause) = else_clause {
        let block = else_clause
            .children(&mut cursor)
            .find(|c| c.kind() == "block");
        state.indent();
        state.println("else");
        state.level += 1;
        if let Some(block) = block {
            format_block(state, block)?;
        } else {
            print_non_linter_comments_after(state, else_clause)?;
        }
        state.level -= 1;
    }
    state.indent();
    state.print("end");
    Ok(())
}

fn format_for(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let parfor = node.child(0).err_at_loc(&node)?.utf8_text(state.code)?;
    state.print(parfor);
    state.print(" ");
    let iterator = node
        .children(&mut cursor)
        .find(|c| c.kind() == "iterator")
        .err_at_loc(&node)?;
    let block = node.children(&mut cursor).find(|c| c.kind() == "block");
    let parfor_options = node
        .children(&mut cursor)
        .find(|c| c.kind() == "parfor_options");
    if let Some(options) = parfor_options {
        state.print("(");
        state.print_node(iterator.named_child(0).err_at_loc(&node)?)?;
        state.print(" = ");
        format_node(state, iterator.named_child(1).err_at_loc(&node)?)?;
        state.print(", ");
        state.print_node(options.named_child(0).err_at_loc(&node)?)?;
        state.print(")");
    } else {
        state.print_node(iterator.named_child(0).err_at_loc(&node)?)?;
        state.print(" = ");
        format_node(state, iterator.named_child(1).err_at_loc(&node)?)?;
    }
    print_linter_comment(state, node)?;
    state.println("");
    state.level += 1;
    if let Some(block) = block {
        format_block(state, block)?;
    } else {
        print_non_linter_comments(state, node)?;
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_function(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let output = node
        .named_children(&mut cursor)
        .find(|n| n.kind() == "function_output");
    let get_set = node
        .children(&mut cursor)
        .filter(|n| !n.is_named())
        .find(|n| match n.utf8_text(state.code) {
            Ok("get.") => true,
            Ok("set.") => true,
            Ok(_) => false,
            Err(_) => false,
        });
    let name = node.child_by_field_name("name").err_at_loc(&node)?;
    let arguments = node
        .named_children(&mut cursor)
        .find(|n| n.kind() == "function_arguments");
    let argument_statements: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "arguments_statement")
        .collect();
    let block = node
        .named_children(&mut cursor)
        .find(|n| n.kind() == "block");
    state.print("function ");
    if let Some(output) = output {
        format_node(state, output.child(0).err_at_loc(&node)?)?;
        state.print(" = ");
    }
    if let Some(get_set) = get_set {
        state.print_node(get_set)?;
        state.print(".");
    }
    state.print_node(name)?;
    if let Some(arguments) = arguments {
        state.print("(");
        let children = arguments
            .named_children(&mut cursor)
            .filter(|c| c.kind() != "line_continuation");
        for (i, arg) in children.enumerate() {
            if i != 0 {
                state.print(", ");
            }
            state.print_node(arg)?;
        }
        state.print(")");
    }
    state.println("");
    state.level += 1;
    for argument_statement in argument_statements {
        state.indent();
        format_node(state, argument_statement)?;
        state.println("");
    }
    if let Some(block) = block {
        format_block(state, block)?;
    } else {
        print_non_linter_comments(state, node)?;
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_arguments_statement(state: &mut State, node: Node) -> Result<()> {
    state.extra_indentation = 0;
    let mut cursor = node.walk();
    let attributes = node
        .children(&mut cursor)
        .find(|c| c.kind() == "attributes");
    let properties = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "property");
    state.print("arguments");
    if let Some(attributes) = attributes {
        state.print(" (");
        format_arguments(state, attributes)?;
        state.print(")");
    }
    state.println("");
    state.level += 1;
    for property in properties {
        state.indent();
        format_property(state, property)?;
        state.println("");
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_property(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let name = node.child_by_field_name("name").err_at_loc(&node)?;
    let dimensions = node
        .children(&mut cursor)
        .find(|c| c.kind() == "dimensions");
    let class = node
        .children(&mut cursor)
        .filter(|c| c.id() != name.id())
        .find(|c| c.kind() == "identifier" || c.kind() == "property_name");
    let validation_functions = node
        .children(&mut cursor)
        .find(|c| c.kind() == "validation_functions");
    let default_value = node
        .children(&mut cursor)
        .find(|c| c.kind() == "default_value");
    if name.kind() == "identifier" {
        state.print_node(name)?;
    } else {
        format_property_name(state, name)?;
    }
    if let Some(dimmensions) = dimensions {
        state.print(" ");
        format_dimensions(state, dimmensions)?;
    }
    if let Some(class) = class {
        state.print(" ");
        format_node(state, class)?;
    }
    if let Some(validation_functions) = validation_functions {
        state.print(" {");
        format_arguments(state, validation_functions)?;
        state.print("}");
    }
    if let Some(default_value) = default_value {
        state.print(" = ");
        format_node(
            state,
            default_value.named_child(0).err_at_loc(&default_value)?,
        )?;
    }
    Ok(())
}

fn format_property_name(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        state.print_node(child)?;
    }
    Ok(())
}

fn format_dimensions(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    state.print("(");
    for (i, child) in node.named_children(&mut cursor).enumerate() {
        if i != 0 {
            state.print(",");
        }
        state.print_node(child)?;
    }
    state.print(")");
    Ok(())
}

fn format_classdef(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let attributes = node
        .children(&mut cursor)
        .find(|c| c.kind() == "attributes");
    let name = node.child_by_field_name("name").err_at_loc(&node)?;
    let superclasses = node
        .children(&mut cursor)
        .find(|c| c.kind() == "superclasses");
    let properties: Vec<Node> = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "properties")
        .collect();
    let methods: Vec<Node> = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "methods")
        .collect();
    let events: Vec<Node> = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "events")
        .collect();
    let enumerations: Vec<Node> = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "enumeration")
        .collect();
    state.print("classdef ");
    if let Some(attributes) = attributes {
        format_attributes(state, attributes)?;
        state.print(" ");
    }
    state.print_node(name)?;
    if let Some(superclasses) = superclasses {
        state.print(" < ");
        for (i, superclass) in superclasses.named_children(&mut cursor).enumerate() {
            if i != 0 {
                state.print(" & ");
            }
            format_property_name(state, superclass)?;
        }
    }
    state.println("");
    state.extra_indentation = 0;
    state.level += 1;
    for property in properties {
        state.indent();
        format_properties(state, property)?;
        state.println("");
    }
    for enumeration in enumerations {
        state.indent();
        format_enum(state, enumeration)?;
        state.println("");
    }
    for event in events {
        state.indent();
        format_events(state, event)?;
        state.println("");
    }
    for method in methods {
        state.indent();
        format_method(state, method)?;
        state.println("");
    }
    state.level -= 1;
    state.print("end");
    Ok(())
}

fn format_attributes(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    state.print("(");
    let attributes = node
        .children(&mut cursor)
        .filter(|c| c.kind() == "attribute");
    for (i, attr) in attributes.enumerate() {
        if i != 0 {
            state.print(", ");
        }
        format_attribute(state, attr)?;
    }
    state.print(")");
    Ok(())
}

fn format_attribute(state: &mut State, node: Node) -> Result<()> {
    let identifier = node.named_child(0).err_at_loc(&node)?;
    let value = node.named_child(1);
    state.print_node(identifier)?;
    if let Some(value) = value {
        state.print("=");
        format_node(state, value)?;
    }
    Ok(())
}

fn format_properties(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let attributes = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "attributes");
    let properties = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "property");
    state.print("properties");
    if let Some(attributes) = attributes {
        state.print(" ");
        format_attributes(state, attributes)?;
    }
    state.println("");
    state.level += 1;
    for property in properties {
        state.indent();
        format_property(state, property)?;
        state.println("");
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_enum(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let attributes = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "attributes");
    let enums: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "enum")
        .collect();
    state.print("enumeration");
    if let Some(attributes) = attributes {
        state.print(" ");
        format_attributes(state, attributes)?;
    }
    state.println("");
    state.level += 1;
    for enumeration in enums {
        state.indent();
        let mut parens = false;
        for (i, c) in enumeration.named_children(&mut cursor).enumerate() {
            if i == 0 {
                state.print_node(c)?;
            } else if i == 1 {
                parens = true;
                state.print(" (");
                format_node(state, c)?;
            } else {
                state.print(", ");
                format_node(state, c)?;
            }
        }
        if parens {
            state.print(")");
        }
        state.println("");
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_events(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let attributes = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "attributes");
    let identifiers = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "identifier");
    state.print("events");
    if let Some(attributes) = attributes {
        state.print(" ");
        format_attributes(state, attributes)?;
    }
    state.println("");
    state.level += 1;
    for identifier in identifiers {
        state.indent();
        state.print_node(identifier)?;
        state.println("");
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_method(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let attributes = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "attributes");
    let definitions: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "function_definition")
        .collect();
    let signatures: Vec<Node> = node
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "function_signature")
        .collect();
    state.print("methods");
    if let Some(attributes) = attributes {
        state.print(" ");
        format_attributes(state, attributes)?;
    }
    state.println("");
    state.level += 1;
    for signature in &signatures {
        state.indent();
        format_signature(state, *signature)?;
        state.println("");
    }
    for (i, definition) in definitions.iter().enumerate() {
        if i != 0 || !signatures.is_empty() {
            state.println("");
        }
        state.indent();
        format_function(state, *definition)?;
        state.println("");
    }
    state.level -= 1;
    state.indent();
    state.print("end");
    Ok(())
}

fn format_signature(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let output = node
        .named_children(&mut cursor)
        .find(|n| n.kind() == "function_output");
    let get_set = node
        .children(&mut cursor)
        .filter(|n| !n.is_named())
        .find(|n| match n.utf8_text(state.code) {
            Ok("get.") => true,
            Ok("set.") => true,
            Ok(_) => false,
            Err(_) => false,
        });
    let name = node.child_by_field_name("name").err_at_loc(&node)?;
    let arguments = node
        .named_children(&mut cursor)
        .find(|n| n.kind() == "function_arguments");
    if let Some(output) = output {
        format_node(state, output.child(0).err_at_loc(&output)?)?;
        state.print(" = ");
    }
    if let Some(get_set) = get_set {
        state.print_node(get_set)?;
        state.print(".");
    }
    state.print_node(name)?;
    if let Some(arguments) = arguments {
        state.print("(");
        let children = arguments
            .named_children(&mut cursor)
            .filter(|c| c.kind() != "line_continuation");
        for (i, arg) in children.enumerate() {
            if i != 0 {
                state.print(", ");
            }
            state.print_node(arg)?;
        }
        state.print(")");
    }
    Ok(())
}

fn print_linter_comment(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let comments = node
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "comment" && n.utf8_text(state.code).unwrap().starts_with("%#"));
    for comment in comments {
        format_comment(state, comment)?;
    }
    Ok(())
}

fn print_non_linter_comments(state: &mut State, node: Node) -> Result<()> {
    let mut cursor = node.walk();
    let comments = node
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "comment" && !n.utf8_text(state.code).unwrap().starts_with("%#"));
    for comment in comments {
        state.indent();
        format_comment(state, comment)?;
        state.println("");
    }
    Ok(())
}

fn print_non_linter_comments_after(state: &mut State, node: Node) -> Result<()> {
    let mut comments: Vec<Node> = vec![];
    let mut cur = node;
    while let Some(next) = cur.next_named_sibling() {
        if next.kind() == "comment" && !next.utf8_text(state.code).unwrap().starts_with("%#") {
            comments.push(next);
            cur = next;
        } else {
            break;
        }
    }
    for comment in comments {
        state.indent();
        format_comment(state, comment)?;
        state.println("");
    }
    Ok(())
}
