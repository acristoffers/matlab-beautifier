mod args;
mod beautifier;

use args::{Arguments, Parser};

fn main() {
    let options = Arguments::parse();
    beautifier::beautify(options);
}
