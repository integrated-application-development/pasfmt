use std::{ops::Range, time::Duration};

use criterion::{criterion_group, criterion_main, Criterion};
use pasfmt_core::prelude::*;
use rand::{seq::SliceRandom, Rng};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer");

    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(15));

    bench_lexer(&mut group, "operators", random_operator);
    bench_lexer(&mut group, "keywords", random_keyword);

    bench_lexer(&mut group, "short identifiers", random_short_ident);
    bench_lexer(&mut group, "medium identifiers", random_medium_ident);
    bench_lexer(&mut group, "long identifiers", random_long_ident);
    bench_lexer(&mut group, "longer identifiers", random_longer_ident);

    bench_lexer(
        &mut group,
        "keywords & identifiers",
        random_keyword_or_ident,
    );

    bench_lexer(&mut group, "short comments", random_short_comment);
    bench_lexer(
        &mut group,
        "long block comments",
        random_long_block_comments,
    );

    bench_lexer(&mut group, "text_literals", random_text_literal);
    bench_lexer(&mut group, "number_literals", random_number_literal);

    bench_lexer(&mut group, "random_spacing", |s| {
        s.push('0');
        random_whitespace_after(s);
    });

    group.finish()
}

fn bench_lexer(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    id: &str,
    setup: impl Fn(&mut String) + Copy,
) {
    const LEN: usize = 1_000_000;
    let setup = || repeat_rand(space_after(setup), LEN);

    group.throughput(criterion::Throughput::Bytes(LEN as u64));
    group.bench_function(id, |b| {
        b.iter_batched_ref(
            setup,
            // criterion doesn't currently support returning something that must be outlived by the input
            // which means that we have to include the time to drop the output which isn't ideal
            |input| {
                DelphiLexer {}.lex(input);
            },
            criterion::BatchSize::LargeInput,
        );
    });
}

fn repeat_rand(f: impl Fn(&mut String), min_len: usize) -> String {
    let mut s = String::new();
    while s.len() < min_len {
        f(&mut s);
    }
    s
}

fn random_whitespace_after(s: &mut String) {
    let rng = &mut rand::thread_rng();
    let count = rng.gen_range(1..5);
    s.extend([' ', '\t', '\n', '\r'].choose_multiple(rng, count));
}

fn space_after(setup: impl Fn(&mut String) + Copy) -> impl Fn(&mut String) + Copy {
    move |s| {
        setup(s);
        s.push(' ');
    }
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

fn random_operator(s: &mut String) {
    s.push_str(OPERATORS.choose(&mut rand::thread_rng()).unwrap());
}

fn random_keyword(s: &mut String) {
    s.push_str(KEYWORDS.choose(&mut rand::thread_rng()).unwrap());
}

fn random_keyword_or_ident(s: &mut String) {
    let rng = &mut rand::thread_rng();
    if rng.gen_bool(0.2) {
        random_keyword(s);
    } else {
        random_medium_ident(s);
    }
}

fn random_ident(s: &mut String, range: Range<usize>) {
    let rng = &mut rand::thread_rng();
    let count = rng.gen_range(range);
    s.extend(ALPHABET.choose_multiple(rng, count));
}

fn random_short_ident(s: &mut String) {
    random_ident(s, 1..5);
}

fn random_medium_ident(s: &mut String) {
    random_ident(s, 10..20);
}

fn random_long_ident(s: &mut String) {
    random_ident(s, 50..80);
}

fn random_longer_ident(s: &mut String) {
    random_ident(s, 200..250);
}

fn random_short_comment(s: &mut String) {
    let comment_delims = [("{", "}"), ("(*", "*)"), ("//", "\n")];

    let rng = &mut rand::thread_rng();
    let (start, end) = comment_delims.choose(rng).unwrap();
    s.push_str(start);

    let count = rng.gen_range(0..100);
    s.extend(ALPHABET.choose_multiple(rng, count).map(|s| s.to_owned()));

    s.push_str(end);
}

fn random_long_block_comments(s: &mut String) {
    let comment_delims = [("{", "}"), ("(*", "*)")];

    let rng = &mut rand::thread_rng();
    let (start, end) = comment_delims.choose(rng).unwrap();
    s.push_str(start);

    let count = rng.gen_range(2900..3000);
    for _ in 0..count {
        s.push(*ALPHABET.choose(rng).unwrap());
    }

    s.push_str(end);
}

const STR_CHARS: [&str; 27] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z", "''",
];

fn random_text_literal(s: &mut String) {
    let rng = &mut rand::thread_rng();
    s.push('\'');
    let count = rng.gen_range(0..100);
    s.extend(STR_CHARS.choose_multiple(rng, count).map(|s| s.to_owned()));
    s.push('\'');
}

fn random_number_literal(s: &mut String) {
    let digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

    let rng = &mut rand::thread_rng();

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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
