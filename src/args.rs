/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use clap::CommandFactory;
pub use clap::Parser;

static LONG_ABOUT: &str = "
matlab-beautifier formats and beautifies MATLAB(R) code.

matlab-beautifier is a somewhat opinionated code beautifier.
It allows a small level of control through arguments or a project-specific file.";

#[derive(Debug, Parser)]
#[command(author, version, about = LONG_ABOUT)]
pub struct Arguments {
    /// File to beautify
    #[arg(global = true)]
    pub file: Option<String>,

    /// Prints spaces around math operators.
    #[arg(global = true, long = "sparse-math")]
    pub sparse_math: bool,
}
