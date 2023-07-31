use pasfmt_orchestrator::command_line::Parser;
use std::{
    env::set_current_dir,
    path::{Path, PathBuf},
    process::{exit, Command},
    time::Duration,
};

use criterion::{criterion_group, criterion_main, Criterion};

use pasfmt::{format_with_settings, FormattingSettings};
use pasfmt_orchestrator::command_line::PasFmtConfiguration;

fn bench_format_submodules(submodules: &[(&str, &PathBuf)], c: &mut Criterion) {
    let mut group = c.benchmark_group("submodules");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    for (name, path) in submodules {
        group.bench_function(*name, |b| {
            b.iter(|| {
                let config =
                    PasFmtConfiguration::parse_from(["".into(), "--write".into(), (*path).clone()]);
                format_with_settings(FormattingSettings::default(), config);
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
    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("benches");
    set_current_dir(&source_dir)
        .unwrap_or_else(|_| panic!("Failed to change into the source directory: {source_dir:?}"));

    init_submodules();

    let submodule_dir = source_dir.join("modules");

    bench_format_submodules(
        &[
            // Currently there are some severe performance issues in other Indy files, so we limit to a subdirectory.
            ("Indy", &submodule_dir.join("Indy/Test")),
            ("DEC", &submodule_dir.join("DelphiEncryptionCompendium")),
        ],
        c,
    );

    init_submodules();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
