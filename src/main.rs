/*
 * How should the project structure look like?
 *We want to create an application, that given a c project, with make/cmake directives,
 *Generate a version of the project where every function call is expanded.
 *With a comment ahead specifying the function.
*/

mod ast;
mod makefile_parser;
mod expander;

fn main() {
    println!("Hello, world!");
}
