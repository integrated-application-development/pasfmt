use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pasfmt_core::prelude::*;
use rand::{seq::SliceRandom, Rng};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer");

    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(10));

    bench_lexer(&mut group, "operators", random_operator);
    bench_lexer(&mut group, "keywords", random_keyword);
    bench_lexer(&mut group, "identifiers", random_ident);
    bench_lexer(&mut group, "comments", random_comment);
    bench_lexer(&mut group, "text_literals", random_text_literal);
    bench_lexer(&mut group, "number_literals", random_number_literal);

    bench_lexer(&mut group, "random_spacing", || {
        random_whitespace_after("0".to_owned())
    });
}

fn bench_lexer<S: Into<String>>(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    id: &str,
    setup: impl Fn() -> S + Copy,
) {
    group.bench_function(id, |b| {
        b.iter_with_setup(|| repeat_rand(|| space_after(setup().into())), run_lexer)
    });
}

fn run_lexer<S: AsRef<str>>(input: S) {
    black_box(DelphiLexer {}.lex(input.as_ref()));
}

fn repeat_rand(f: impl Fn() -> String) -> String {
    // each case needs to sample from the random generator a large number of
    // times to reduce the noise that it introduces
    std::iter::repeat_with(f).take(100000).collect::<String>()
}

fn random_whitespace_after(mut s: String) -> String {
    let rng = &mut rand::thread_rng();
    let count = rng.gen_range(1..5);
    s.extend([' ', '\t', '\n', '\r'].choose_multiple(rng, count));
    s
}

fn space_after(mut s: String) -> String {
    s.push(' ');
    s
}

const ALPHABET: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

const OPERATORS: [&str; 24] = [
    "+", "-", "*", "/", ":=", ",", ";", ":", "=", "<>", "<=", "<", ">=", ">", "(.", ".)", "[", "]",
    "(", ")", "^", "@", "..", ".",
];

const KEYWORDS: [&str; 123] = [
    "absolute",
    "abstract",
    "add",
    "align",
    "and",
    "array",
    "as",
    "asm",
    "assembler",
    "at",
    "automated",
    "begin",
    "case",
    "cdecl",
    "class",
    "const",
    "constructor",
    "contains",
    "default",
    "delayed",
    "deprecated",
    "destructor",
    "dispid",
    "dispinterface",
    "div",
    "do",
    "downto",
    "dynamic",
    "else",
    "end",
    "except",
    "experimental",
    "export",
    "exports",
    "external",
    "far",
    "file",
    "final",
    "finalization",
    "finally",
    "for",
    "forward",
    "function",
    "goto",
    "helper",
    "if",
    "implementation",
    "implements",
    "in",
    "index",
    "inherited",
    "initialization",
    "inline",
    "interface",
    "is",
    "label",
    "library",
    "local",
    "message",
    "mod",
    "name",
    "near",
    "nil",
    "nodefault",
    "not",
    "object",
    "of",
    "on",
    "operator",
    "or",
    "out",
    "overload",
    "override",
    "package",
    "packed",
    "pascal",
    "platform",
    "private",
    "procedure",
    "program",
    "property",
    "protected",
    "public",
    "published",
    "raise",
    "read",
    "readonly",
    "record",
    "reference",
    "register",
    "reintroduce",
    "remove",
    "repeat",
    "requires",
    "resident",
    "resourcestring",
    "safecall",
    "sealed",
    "set",
    "shl",
    "shr",
    "static",
    "stdcall",
    "stored",
    "strict",
    "then",
    "threadvar",
    "to",
    "try",
    "type",
    "unit",
    "unsafe",
    "until",
    "uses",
    "var",
    "varargs",
    "variant",
    "virtual",
    "while",
    "with",
    "write",
    "writeonly",
    "xor",
];

fn random_operator() -> &'static str {
    OPERATORS.choose(&mut rand::thread_rng()).unwrap()
}

fn random_keyword() -> &'static str {
    KEYWORDS.choose(&mut rand::thread_rng()).unwrap()
}

fn random_ident() -> String {
    let rng = &mut rand::thread_rng();
    let count = rng.gen_range(0..100);
    ALPHABET.choose_multiple(rng, count).collect::<String>()
}

fn random_comment() -> String {
    let comment_delims = [("{", "}"), ("(*", "*)"), ("//", "\n")];

    let rng = &mut rand::thread_rng();
    let (start, end) = comment_delims.choose(rng).unwrap();
    let mut s = String::from(*start);

    let count = rng.gen_range(0..100);
    s.extend(ALPHABET.choose_multiple(rng, count).map(|s| s.to_owned()));

    s.push_str(end);

    s
}

const STR_CHARS: [&str; 27] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z", "''",
];

fn random_text_literal() -> String {
    let rng = &mut rand::thread_rng();
    let mut s = String::from("'");
    let count = rng.gen_range(0..100);
    s.extend(STR_CHARS.choose_multiple(rng, count).map(|s| s.to_owned()));
    s.push('\'');
    s
}

fn random_number_literal() -> String {
    let digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

    let rng = &mut rand::thread_rng();
    let mut s = String::new();

    let num_digits = rng.gen_range(1..15) as usize;
    s.extend(
        digits
            .choose_multiple(rng, num_digits)
            .map(|s| s.to_owned()),
    );

    if rng.gen_bool(0.2) {
        s.push('.');
        let num_digits = rng.gen_range(1..5) as usize;
        s.extend(
            digits
                .choose_multiple(rng, num_digits)
                .map(|s| s.to_owned()),
        );
    }

    if rng.gen_bool(0.1) {
        s.push('e');
        s.push(*['+', '-'].choose(rng).unwrap());
        let num_digits = rng.gen_range(1..3) as usize;
        s.extend(
            digits
                .choose_multiple(rng, num_digits)
                .map(|s| s.to_owned()),
        );
    }

    s
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
