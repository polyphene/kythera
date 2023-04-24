## Basic Test Actor

This is a basic actor to test our Kythera testing framework. It has two main entry points: `TestOne()` and `TestTwo()`.

Their entry point values, respectively `3948827889` and `891686990` have been calculated through the FRC 0042 and Helix 
`frc42_dispatch` crate. We wanted to use the `frc42_dispatch::match_method!` macro to generate proper method number value at compilation
time but we faced dependencies issue that prevented us from doing so for now.