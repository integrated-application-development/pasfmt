use predicates::{
    ord::EqPredicate,
    prelude::*,
    str::{NormalizedPredicate, TrimPredicate},
};

use crate::utils::*;

fn eq_long_help_no_col() -> NormalizedPredicate<TrimPredicate<EqPredicate<&'static str>>> {
    predicate::eq(include_str!("data/help/long_help_no_col.txt").trim())
        .trim()
        .normalize()
}

fn eq_short_help_no_col() -> NormalizedPredicate<TrimPredicate<EqPredicate<&'static str>>> {
    predicate::eq(include_str!("data/help/help_no_col.txt").trim())
        .trim()
        .normalize()
}

fn eq_config_help_no_col() -> NormalizedPredicate<TrimPredicate<EqPredicate<&'static str>>> {
    predicate::eq(include_str!("data/help/config_help_no_col.txt").trim())
        .trim()
        .normalize()
}

fn eq_long_help_col() -> NormalizedPredicate<TrimPredicate<EqPredicate<&'static str>>> {
    predicate::eq(include_str!("data/help/long_help_col.txt").trim())
        .trim()
        .normalize()
}

fn eq_short_help_col() -> NormalizedPredicate<TrimPredicate<EqPredicate<&'static str>>> {
    predicate::eq(include_str!("data/help/help_col.txt").trim())
        .trim()
        .normalize()
}

fn eq_config_help_col() -> NormalizedPredicate<TrimPredicate<EqPredicate<&'static str>>> {
    predicate::eq(include_str!("data/help/config_help_col.txt").trim())
        .trim()
        .normalize()
}

#[test]
fn short_help_no_col() -> TestResult {
    let mut cmd = pasfmt()?;
    cmd.arg("-h").env("NO_COLOR", "1");
    cmd.assert().success().stdout(eq_short_help_no_col());

    Ok(())
}

#[test]
fn short_help_col() -> TestResult {
    let mut cmd = pasfmt()?;
    cmd.arg("-h").env("CLICOLOR_FORCE", "1");
    cmd.assert().success().stdout(eq_short_help_col());

    Ok(())
}

#[test]
fn long_help_no_col() -> TestResult {
    let mut cmd = pasfmt()?;

    cmd.arg("--help").env("NO_COLOR", "1");
    cmd.assert().success().stdout(eq_long_help_no_col());

    Ok(())
}

#[test]
fn long_help_col() -> TestResult {
    let mut cmd = pasfmt()?;
    cmd.arg("--help").env("CLICOLOR_FORCE", "1");
    cmd.assert().success().stdout(eq_long_help_col());

    Ok(())
}

#[test]
fn config_help_col() -> TestResult {
    let mut cmd = pasfmt()?;
    cmd.arg("-Chelp").env("CLICOLOR_FORCE", "1");
    cmd.assert().success().stdout(eq_config_help_col());

    Ok(())
}

#[test]
fn config_help_no_col() -> TestResult {
    let mut cmd = pasfmt()?;
    cmd.arg("-Chelp").env("NO_COLOR", "1");
    cmd.assert().success().stdout(eq_config_help_no_col());

    Ok(())
}
