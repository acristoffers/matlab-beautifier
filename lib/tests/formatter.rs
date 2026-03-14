/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Fixture-based regression tests for the MATLAB beautifier.
//!
//! Each test loads a `.m` file from `tests/fixtures/`, formats it, and asserts
//! idempotency: the formatter must reproduce the file unchanged. This catches
//! regressions introduced by updates to tree-sitter-matlab (grammar node
//! renames, structural changes, new node types, etc.).
//!
//! To add a new test:
//!   1. Create `tests/fixtures/<name>.m` with valid, already-formatted MATLAB.
//!   2. Add `fixture_test!(test_<name>, "<name>.m");` below.

use matlab_beautifier::{beautify, Arguments};

fn make_args() -> Arguments {
    Arguments {
        files: vec![],
        sparse_math: false,
        sparse_add: false,
        inplace: true,
    }
}

fn assert_idempotent(fixture_name: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture_name);

    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Could not read fixture '{}': {}", fixture_name, e));

    let mut args = make_args();
    let result = beautify(&content, &mut args)
        .unwrap_or_else(|e| panic!("beautify() failed for '{}': {}", fixture_name, e));

    if content != result {
        // Build a simple line-diff to make failures easy to diagnose.
        let expected_lines: Vec<&str> = content.lines().collect();
        let got_lines: Vec<&str> = result.lines().collect();
        let mut diff = String::new();
        let max = expected_lines.len().max(got_lines.len());
        for i in 0..max {
            let exp = expected_lines.get(i).copied().unwrap_or("<missing>");
            let got = got_lines.get(i).copied().unwrap_or("<missing>");
            if exp != got {
                diff.push_str(&format!("  line {:>3} expected: {:?}\n", i + 1, exp));
                diff.push_str(&format!("  line {:>3}      got: {:?}\n", i + 1, got));
            }
        }
        panic!(
            "Formatter output for '{}' is not idempotent.\nDiffs:\n{}",
            fixture_name, diff
        );
    }
}

macro_rules! fixture_test {
    ($fn_name:ident, $file:expr) => {
        #[test]
        fn $fn_name() {
            assert_idempotent($file);
        }
    };
}

// -- Expressions and operators ------------------------------------------------
fixture_test!(test_assignment, "assignment.m");
fixture_test!(test_binary_operators, "binary_operators.m");
fixture_test!(test_boolean_operators, "boolean_operators.m");
fixture_test!(test_operators, "operators.m");
fixture_test!(test_range, "range.m");
fixture_test!(test_lambda, "lambda.m");

// -- Compound expressions -----------------------------------------------------
fixture_test!(test_matrix_cell, "matrix_cell.m");
fixture_test!(test_field_expression, "field_expression.m");
fixture_test!(test_function_call, "function_call.m");

// -- Control flow -------------------------------------------------------------
fixture_test!(test_if_statement, "if_statement.m");
fixture_test!(test_for_statement, "for_statement.m");
fixture_test!(test_while_statement, "while_statement.m");
fixture_test!(test_switch_statement, "switch_statement.m");
fixture_test!(test_try_statement, "try_statement.m");
fixture_test!(test_spmd_statement, "spmd_statement.m");

// -- Functions and arguments --------------------------------------------------
fixture_test!(test_function_definition, "function_definition.m");
fixture_test!(test_arguments_block, "arguments_block.m");
fixture_test!(test_global_persistent, "global_persistent.m");

// -- Classes ------------------------------------------------------------------
fixture_test!(test_class_definition, "class_definition.m");
fixture_test!(test_property_name, "property_name.m");

// -- Miscellaneous ------------------------------------------------------------
fixture_test!(test_comment, "comment.m");
fixture_test!(test_command, "command.m");
fixture_test!(test_line_continuation, "line_continuation.m");
