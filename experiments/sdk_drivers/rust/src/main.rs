use std::env;
use std::fs;

use fathomdb_sdk_benchmark_driver::handle_request;

fn main() {
    let request = match env::args().collect::<Vec<_>>().as_slice() {
        [_, path] => fs::read_to_string(path).unwrap_or_default(),
        _ => String::new(),
    };
    println!("{}", handle_request(&request));
}
