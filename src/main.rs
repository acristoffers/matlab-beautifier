/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod args;
mod beautifier;

use args::{Arguments, Parser};
use std::io::Read;

use self::beautifier::beautify;

fn main() {
    let mut options = Arguments::parse();
    if options.files.is_empty() {
        options.inplace = false;
        beautify_file(None, &mut options);
    } else {
        if options.files.len() > 1 {
            options.inplace = true;
        }
        let files = options.files.clone();
        for file in files {
            if options.inplace {
                print!("Formatting file {}: ", file);
            }
            beautify_file(Some(file), &mut options);
        }
    }
}

fn beautify_file(file: Option<String>, options: &mut Arguments) {
    let mut code: String = "".into();
    match &file {
        Some(file) => {
            code = std::fs::read_to_string(file).expect("Could not read file.");
        }
        None => {
            std::io::stdin()
                .read_to_string(&mut code)
                .expect("Error reading from stdin.");
        }
    }
    let result = beautify(code.as_str(), options);
    if options.inplace {
        if let Some(formatted) = result {
            print!("file formatted ");
            match std::fs::write(file.unwrap().as_str(), formatted.as_bytes()) {
                Ok(_) => println!("and overwritten."),
                Err(_) => println!("but could not write back."),
            }
        } else {
            println!("could not format file.");
        }
    } else if result.is_none() {
        panic!("Could not format file.");
    }
}
