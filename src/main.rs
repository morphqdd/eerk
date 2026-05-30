use clap::{Parser, Subcommand};
use eerk::{files, manifest, policy, render, report::Report};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "eerk")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare a repo against the baseline and write report.json.
    Check {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        state: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Render a report.json to markdown on stdout.
    Render {
        #[arg(long)]
        report: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            baseline,
            manifest: manifest_path,
            repo,
            state,
            out,
        } => {
            let m = manifest::parse(&fs::read_to_string(&manifest_path).expect("read manifest"))
                .expect("parse manifest");
            let state_map: policy::State =
                serde_json::from_str(&fs::read_to_string(&state).expect("read state"))
                    .expect("parse state.json");

            let files_findings = files::check(&m.file, &baseline, &repo);
            let policy_findings: Vec<_> = policy::check(&m.policy, &state_map)
                .into_iter()
                .filter(|f| f.status != eerk::report::PolicyStatus::Ok)
                .collect();

            let report = Report {
                files: files_findings,
                policy: policy_findings,
            };
            fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).expect("write report");

            if report.has_drift() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Command::Render { report } => {
            let r: Report =
                serde_json::from_str(&fs::read_to_string(&report).expect("read report"))
                    .expect("parse report.json");
            print!("{}", render::render(&r));
            ExitCode::SUCCESS
        }
    }
}
