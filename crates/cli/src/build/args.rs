use super::common::slugify;

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

pub(crate) struct BuildMapOptions {
    pub(crate) id: Option<String>,
    pub(crate) content_dir: Option<String>,
    pub(crate) used_content_dir: bool,
}

impl BuildMapOptions {
    pub(crate) fn from_args(args: &[String]) -> Self {
        let content_dir = flag_value(args, "--content");
        let used_content_dir = flag_value(args, "--content-dir").is_some();
        let positionals = collect_positionals(args);
        let id = positionals.first().map(|value| slugify(value));
        Self {
            id,
            content_dir,
            used_content_dir,
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
