use clap::{Args, Subcommand, ValueHint};

use super::common::slugify;

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum BuildCommand {
    New(BuildNewArgs),
    Map(BuildMapArgs),
    Upgrade(BuildUpgradeArgs),
    Strings(BuildStringsArgs),
    NewProject(BuildNewProjectArgs),
    Docs(BuildDocsArgs),
}

#[derive(Clone, Debug, Args)]
pub(crate) struct BuildNewArgs {
    pub(crate) kind: String,
    pub(crate) id: String,
    #[arg(long)]
    pub(crate) name: Option<String>,
    #[arg(long, value_hint = ValueHint::AnyPath)]
    pub(crate) content: Option<String>,
    #[arg(long = "content-dir", value_hint = ValueHint::AnyPath)]
    pub(crate) content_dir: Option<String>,
    #[arg(long)]
    pub(crate) force: bool,
}

impl BuildNewArgs {
    pub(crate) fn to_argv(&self) -> Vec<String> {
        let mut out = vec![self.kind.clone(), self.id.clone()];
        push_opt_flag(&mut out, "--name", self.name.as_deref());
        push_opt_flag(&mut out, "--content", self.content.as_deref());
        push_opt_flag(&mut out, "--content-dir", self.content_dir.as_deref());
        push_bool_flag(&mut out, "--force", self.force);
        out
    }
}

#[derive(Clone, Debug, Args)]
pub(crate) struct BuildMapArgs {
    pub(crate) id: String,
    #[arg(long, value_hint = ValueHint::AnyPath)]
    pub(crate) content: Option<String>,
    #[arg(long = "content-dir", hide = true, value_hint = ValueHint::AnyPath)]
    pub(crate) content_dir: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub(crate) struct BuildUpgradeArgs {
    #[arg(long, value_hint = ValueHint::AnyPath)]
    pub(crate) content: Option<String>,
    #[arg(long = "content-dir", value_hint = ValueHint::AnyPath)]
    pub(crate) content_dir: Option<String>,
    #[arg(long = "dry-run")]
    pub(crate) dry_run: bool,
}

impl BuildUpgradeArgs {
    pub(crate) fn to_argv(&self) -> Vec<String> {
        let mut out = Vec::new();
        push_opt_flag(&mut out, "--content", self.content.as_deref());
        push_opt_flag(&mut out, "--content-dir", self.content_dir.as_deref());
        push_bool_flag(&mut out, "--dry-run", self.dry_run);
        out
    }
}

#[derive(Clone, Debug, Args)]
pub(crate) struct BuildStringsArgs {
    #[arg(long, value_hint = ValueHint::AnyPath)]
    pub(crate) content: Option<String>,
    #[arg(long = "content-dir", value_hint = ValueHint::AnyPath)]
    pub(crate) content_dir: Option<String>,
    #[arg(long)]
    pub(crate) force: bool,
}

impl BuildStringsArgs {
    pub(crate) fn to_argv(&self) -> Vec<String> {
        let mut out = Vec::new();
        push_opt_flag(&mut out, "--content", self.content.as_deref());
        push_opt_flag(&mut out, "--content-dir", self.content_dir.as_deref());
        push_bool_flag(&mut out, "--force", self.force);
        out
    }
}

#[derive(Clone, Debug, Args)]
pub(crate) struct BuildNewProjectArgs {
    pub(crate) name: String,
    #[arg(long, value_hint = ValueHint::AnyPath)]
    pub(crate) path: Option<String>,
}

impl BuildNewProjectArgs {
    pub(crate) fn to_argv(&self) -> Vec<String> {
        let mut out = vec![self.name.clone()];
        push_opt_flag(&mut out, "--path", self.path.as_deref());
        out
    }
}

#[derive(Clone, Debug, Args)]
pub(crate) struct BuildDocsArgs {
    #[arg(short = 's', long)]
    pub(crate) schemas: bool,
    #[arg(short = 'a', long)]
    pub(crate) architecture: bool,
    #[arg(short = 'c', long = "content-authoring")]
    pub(crate) content_authoring: bool,
    #[arg(short = 'j', long)]
    pub(crate) jobs: bool,
}

impl BuildDocsArgs {
    pub(crate) fn to_argv(&self) -> Vec<String> {
        let mut out = Vec::new();
        push_bool_flag(&mut out, "--schemas", self.schemas);
        push_bool_flag(&mut out, "--architecture", self.architecture);
        push_bool_flag(&mut out, "--content-authoring", self.content_authoring);
        push_bool_flag(&mut out, "--jobs", self.jobs);
        out
    }
}

fn push_opt_flag(out: &mut Vec<String>, flag: &str, value: Option<&str>) {
    if let Some(value) = value {
        out.push(flag.to_string());
        out.push(value.to_string());
    }
}

fn push_bool_flag(out: &mut Vec<String>, flag: &str, enabled: bool) {
    if enabled {
        out.push(flag.to_string());
    }
}

pub(crate) struct BuildNewOptions {
    pub(crate) kind: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) content_dir: Option<String>,
    pub(crate) force: bool,
}

impl BuildNewOptions {
    pub(crate) fn from_args(args: &[String]) -> Self {
        let content_dir =
            flag_value(args, "--content").or_else(|| flag_value(args, "--content-dir"));
        let name = flag_value(args, "--name");
        let force = has_flag(args, "--force");
        let positionals = collect_positionals(args);
        let kind = positionals.first().cloned();
        let id = positionals.get(1).map(|value| slugify(value));
        Self {
            kind,
            id,
            name,
            content_dir,
            force,
        }
    }
}

pub(crate) struct BuildUpgradeOptions {
    pub(crate) content_dir: Option<String>,
    pub(crate) dry_run: bool,
}

impl BuildUpgradeOptions {
    pub(crate) fn from_args(args: &[String]) -> Self {
        Self {
            content_dir: flag_value(args, "--content")
                .or_else(|| flag_value(args, "--content-dir")),
            dry_run: has_flag(args, "--dry-run"),
        }
    }
}

pub(crate) struct BuildStringsOptions {
    pub(crate) content_dir: Option<String>,
    pub(crate) force: bool,
}

impl BuildStringsOptions {
    pub(crate) fn from_args(args: &[String]) -> Self {
        Self {
            content_dir: flag_value(args, "--content")
                .or_else(|| flag_value(args, "--content-dir")),
            force: has_flag(args, "--force"),
        }
    }
}

pub(crate) struct BuildNewProjectOptions {
    pub(crate) name: Option<String>,
    pub(crate) path: Option<String>,
}

impl BuildNewProjectOptions {
    pub(crate) fn from_args(args: &[String]) -> Self {
        let path = flag_value(args, "--path");
        let positionals = collect_positionals(args);
        let name = positionals.first().cloned();
        Self { name, path }
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix(&format!("{}=", flag)) {
            return Some(value.to_string());
        }
        if arg == flag {
            return iter.next().cloned();
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn collect_positionals(args: &[String]) -> Vec<String> {
    let mut positionals = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg.starts_with("--") {
            if arg == "--force" || arg == "--dry-run" {
                continue;
            }
            if arg.contains('=') {
                continue;
            }
            if arg == "--content" || arg == "--content-dir" || arg == "--name" || arg == "--path" {
                iter.next();
            }
            continue;
        }
        positionals.push(arg.to_string());
    }
    positionals
}
