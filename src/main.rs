/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod args;
mod beautifier;

use anyhow::{Context, Result};
use args::{Arguments, Parser};
use std::io::Read;

use self::beautifier::beautify;

fn main() {
    let mut options = Arguments::parse();
    if options.files.is_empty() {
        options.inplace = false;
        beautify_file(None, &mut options).unwrap();
    } else {
        options.inplace = options.files.len() > 1;
        let files = options.files.clone();
        for file in files {
            if options.inplace {
                print!("Formatting file {}: ", file);
            }
            let r = beautify_file(Some(file), &mut options);
            if let (false, Err(_)) = (options.inplace, &r) {
                r.unwrap()
            } else if let Err(err) = r {
                println!("could not format ({})", err);
            }
        }
    }
}

fn beautify_file(file: Option<String>, options: &mut Arguments) -> Result<()> {
    let mut code: String = "".into();
    if let Some(file) = &file {
        code = std::fs::read_to_string(file).with_context(|| "Could not read file.")?;
    } else {
        std::io::stdin()
            .read_to_string(&mut code)
            .with_context(|| "Could not read from stdin.")?;
    }
    let result = beautify(code.as_str(), options)?;
    if options.inplace {
        print!("file formatted ");
        match std::fs::write(file.unwrap().as_str(), result.as_bytes()) {
            Ok(_) => println!("and overwritten."),
            Err(_) => println!("but could not write back."),
        }
    }
    Ok(())
}
