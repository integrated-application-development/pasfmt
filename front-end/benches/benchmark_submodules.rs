use pasfmt_core::{prelude::DelphiLexer, traits::Lexer};
use pasfmt_orchestrator::command_line::Parser;
use std::{
    env::set_current_dir,
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    process::{exit, Command},
    time::Duration,
};
use walkdir::WalkDir;

use criterion::{criterion_group, criterion_main, Criterion};

use pasfmt::{format, FormattingConfig};
use pasfmt_orchestrator::predule::*;

pasfmt_config!(Config<FormattingConfig>);

fn bench_format_submodules(submodules: &[(&str, &PathBuf)], c: &mut Criterion) {
    let mut group = c.benchmark_group("format_submodules");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    for (name, path) in submodules {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let config = Config::parse_from(["".into(), (*path).clone()]).config;
                format(config, |e| panic!("{e:?}"));
            });
        });
    }

    group.finish();
}

fn bench_lex_submodules(submodules: &[(&str, &PathBuf)], c: &mut Criterion) {
    let mut group = c.benchmark_group("lex_submodules");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(20));

    for (name, path) in submodules {
        let inputs: Vec<String> = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| {
                let e = e.unwrap();
                if e.path().is_dir() {
                    return None;
                }
                e.path()
                    .extension()
                    .filter(|ext| ext.eq_ignore_ascii_case("pas"))?;

                let mut file_bytes = Vec::new();
                let mut file = OpenOptions::new().read(true).open(e.path()).unwrap();
                file.read_to_end(&mut file_bytes).unwrap();

                Some(encoding_rs::WINDOWS_1252.decode(&file_bytes).0.into_owned())
            })
            .collect();

        let total_bytes: usize = inputs.iter().map(|i| i.len()).sum();
        group.throughput(criterion::Throughput::Bytes(total_bytes as u64));
        group.bench_function(*name, |b| {
            b.iter(|| {
                inputs.iter().for_each(|input| {
                    DelphiLexer {}.lex(input);
                })
            });
        });
    }

    group.finish();
}

fn execute_command(mut command: Command) {
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("failed to launch subprocess: {command:?}. {e}"));

    if !output.status.success() {
        eprintln!(
            "Command `{:?}` failed with exit code {}. Stderr:\n{}",
            command,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        exit(1);
    }
}

fn init_submodules() {
    let mut command = Command::new("git");
    command
        .arg("submodule")
        .arg("update")
        .arg("--init")
        .arg("--checkout")
        .arg("--force");
    execute_command(command);
}

fn criterion_benchmark(c: &mut Criterion) {
    let submodule_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/modules");
    set_current_dir(&submodule_dir).unwrap_or_else(|_| {
        panic!("failed to change into the source directory: {submodule_dir:?}")
    });

    init_submodules();

    bench_format_submodules(
        &[
            ("Indy", &submodule_dir.join("Indy")),
            ("DEC", &submodule_dir.join("DelphiEncryptionCompendium")),
        ],
        c,
    );

    bench_lex_submodules(
        &[
            ("Indy", &submodule_dir.join("Indy")),
            ("DEC", &submodule_dir.join("DelphiEncryptionCompendium")),
        ],
        c,
    );

    init_submodules();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
