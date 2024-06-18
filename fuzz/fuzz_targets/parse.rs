#![no_main]

use std::hint::black_box;

use libfuzzer_sys::fuzz_target;
use pasfmt_core::prelude::*;

fuzz_target!(|input: String| {
    let tokens = DelphiLexer {}.lex(&input);
    black_box(DelphiLogicalLineParser {}.parse(tokens));
});
