use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Args {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Runs `cargo clippy`.
    Clippy(ClippyArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        CliCommand::Clippy(args) => run_clippy(args),
    }
}

#[derive(Parser)]
struct ClippyArgs {
    /// Automatically apply lint suggestions (`clippy --fix`).
    #[arg(long)]
    fix: bool,

    /// The package to run Clippy against (`cargo -p <PACKAGE> clippy`).
    #[arg(long, short)]
    package: Option<String>,
}

fn run_clippy(args: ClippyArgs) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut clippy_command = Command::new(&cargo);
    clippy_command.arg("clippy");

    if let Some(package) = args.package {
        clippy_command.args(["--package", &package]);
    } else {
        clippy_command.arg("--workspace");
    }

    clippy_command
        .arg("--release")
        .arg("--all-targets")
        .arg("--all-features");

    if args.fix {
        clippy_command.arg("--fix");
    }

    clippy_command.arg("--");

    // Deny all warnings.
    // We don't do this yet on Windows, as it still has some warnings present.
    #[cfg(not(target_os = "windows"))]
    clippy_command.args(["--deny", "warnings"]);

    /// These are all of the rules that currently have violations in the Zed
    /// codebase.
    ///
    /// We'll want to drive this list down by either:
    /// 1. fixing violations of the rule and begin enforcing it
    /// 2. deciding we want to allow the rule permanently, at which point
    ///    we should codify that separately in this script.
    const MIGRATORY_RULES_TO_ALLOW: &[&str] = &[
        // There's a bunch of rules currently failing in the `style` group, so
        // allow all of those, for now.
        "clippy::style",
        // Individual rules that have violations in the codebase:
        "clippy::almost_complete_range",
        "clippy::arc_with_non_send_sync",
        "clippy::await_holding_lock",
        "clippy::bool_comparison",
        "clippy::borrow_deref_ref",
        "clippy::borrowed_box",
        "clippy::cast_abs_to_unsigned",
        "clippy::clone_on_copy",
        "clippy::cmp_owned",
        "clippy::crate_in_macro_def",
        "clippy::default_constructed_unit_structs",
        "clippy::derivable_impls",
        "clippy::derive_ord_xor_partial_ord",
        "clippy::drain_collect",
        "clippy::eq_op",
        "clippy::expect_fun_call",
        "clippy::explicit_auto_deref",
        "clippy::explicit_counter_loop",
        "clippy::extra_unused_lifetimes",
        "clippy::filter_map_identity",
        "clippy::identity_op",
        "clippy::implied_bounds_in_impls",
        "clippy::iter_kv_map",
        "clippy::iter_overeager_cloned",
        "clippy::let_underscore_future",
        "clippy::manual_find",
        "clippy::manual_flatten",
        "clippy::map_entry",
        "clippy::map_flatten",
        "clippy::map_identity",
        "clippy::needless_arbitrary_self_type",
        "clippy::needless_borrowed_reference",
        "clippy::needless_lifetimes",
        "clippy::needless_option_as_deref",
        "clippy::needless_question_mark",
        "clippy::needless_update",
        "clippy::never_loop",
        "clippy::non_canonical_clone_impl",
        "clippy::non_canonical_partial_ord_impl",
        "clippy::nonminimal_bool",
        "clippy::option_as_ref_deref",
        "clippy::option_map_unit_fn",
        "clippy::redundant_closure_call",
        "clippy::redundant_guards",
        "clippy::redundant_locals",
        "clippy::reversed_empty_ranges",
        "clippy::search_is_some",
        "clippy::single_char_pattern",
        "clippy::single_range_in_vec_init",
        "clippy::suspicious_to_owned",
        "clippy::to_string_in_format_args",
        "clippy::too_many_arguments",
        "clippy::type_complexity",
        "clippy::unit_arg",
        "clippy::unnecessary_cast",
        "clippy::unnecessary_filter_map",
        "clippy::unnecessary_find_map",
        "clippy::unnecessary_operation",
        "clippy::unnecessary_to_owned",
        "clippy::unnecessary_unwrap",
        "clippy::useless_conversion",
        "clippy::useless_format",
        "clippy::vec_init_then_push",
    ];

    // When fixing violations automatically we don't care about the
    // rules we're already violating, since it may be possible to
    // have them fixed automatically.
    if !args.fix {
        for rule in MIGRATORY_RULES_TO_ALLOW {
            clippy_command.args(["--allow", rule]);
        }
    }

    // Deny `dbg!` and `todo!`s.
    clippy_command
        .args(["--deny", "clippy::dbg_macro"])
        .args(["--deny", "clippy::todo"]);

    eprintln!(
        "running: {cargo} {}",
        clippy_command
            .get_args()
            .map(|arg| arg.to_str().unwrap())
            .collect::<Vec<_>>()
            .join(" ")
    );

    let exit_status = clippy_command
        .spawn()
        .context("failed to spawn child process")?
        .wait()
        .context("failed to wait for child process")?;

    if !exit_status.success() {
        bail!("clippy failed: {}", exit_status);
    }

    Ok(())
}
