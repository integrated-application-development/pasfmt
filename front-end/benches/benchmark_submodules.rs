use pasfmt_core::{prelude::DelphiLexer, traits::Lexer};
use pasfmt_orchestrator::command_line::Parser;
use std::{
    env::set_current_dir,
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, exit},
    time::Duration,
};
use walkdir::WalkDir;

use criterion::{Criterion, criterion_group, criterion_main};

use pasfmt::{FormattingConfig, format};
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

fn execute_command(command: &mut Command) {
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

fn clone_repo(url: &str, sha: &str) {
    eprintln!("Cloning {url} ({sha})");
    let clone_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("benches/clones")
        .join(url.split('/').next_back().expect("URL should contain '/'"));
    execute_command(Command::new("git").arg("init").arg(&clone_dir));

    if Command::new("git")
        .current_dir(&clone_dir)
        .args(["remote", "set-url", "origin", url])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_none()
    {
        execute_command(
            Command::new("git")
                .current_dir(&clone_dir)
                .args(["remote", "add", "origin", url]),
        );
    }
    execute_command(Command::new("git").current_dir(&clone_dir).args([
        "fetch",
        "--depth=1",
        "origin",
        sha,
    ]));
    execute_command(
        Command::new("git")
            .current_dir(&clone_dir)
            .args(["reset", "--hard", sha]),
    );
}

fn init_clones() {
    clone_repo(
        "https://github.com/IndySockets/Indy",
        "0d6819aa8eaf6afcae9112ea6b8ca8dfa2d7be05",
    );
    clone_repo(
        "https://github.com/MHumm/DelphiEncryptionCompendium",
        "91d6e66bdb900c23f7e852401c579cc4a7f6a0db",
    );
}

fn criterion_benchmark(c: &mut Criterion) {
    let clones_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/clones");
    set_current_dir(&clones_dir)
        .unwrap_or_else(|_| panic!("failed to change into the source directory: {clones_dir:?}"));

    init_clones();

    bench_format_submodules(
        &[
            ("Indy", &clones_dir.join("Indy")),
            ("DEC", &clones_dir.join("DelphiEncryptionCompendium")),
        ],
        c,
    );

    bench_lex_submodules(
        &[
            ("Indy", &clones_dir.join("Indy")),
            ("DEC", &clones_dir.join("DelphiEncryptionCompendium")),
        ],
        c,
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
