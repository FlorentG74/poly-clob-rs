use polars::prelude::*;
use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
}
