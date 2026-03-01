const ARCHITECTURE_DOC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../ARCHITECTURE.md"
));
const SCHEMAS_DOC: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../SCHEMAS.md"));
const CONTENT_AUTHORING_DOC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../CONTENT_AUTHORING_GUIDE.md"
));
const JOBS_DOC: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../JOBS.md"));

pub(crate) fn run_build_docs(args: &[String]) {
    let mut include_schemas = false;
    let mut include_architecture = false;
    let mut include_content_authoring = false;
    let mut include_jobs = false;
    let mut unknown = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-s" | "--schemas" => include_schemas = true,
            "-a" | "--architecture" => include_architecture = true,
            "-c" | "--content-authoring" => include_content_authoring = true,
            "-j" | "--jobs" => include_jobs = true,
            _ => unknown.push(arg.clone()),
        }
    }

    if !unknown.is_empty() {
        eprintln!("Unknown docs flags: {}", unknown.join(" "));
        super::print_build_usage();
        return;
    }

    if !(include_schemas || include_architecture || include_content_authoring || include_jobs) {
        include_schemas = true;
        include_architecture = true;
        include_content_authoring = true;
        include_jobs = true;
    }

    let mut wrote_any = false;
    let mut print_doc = |name: &str, content: &str| {
        if wrote_any {
            println!();
        }
        println!("----- {} -----", name);
        print!("{}", content);
        if !content.ends_with('\n') {
            println!();
        }
        wrote_any = true;
    };

    if include_architecture {
        print_doc("ARCHITECTURE.md", ARCHITECTURE_DOC);
    }
    if include_schemas {
        print_doc("SCHEMAS.md", SCHEMAS_DOC);
    }
    if include_content_authoring {
        print_doc("CONTENT_AUTHORING_GUIDE.md", CONTENT_AUTHORING_DOC);
    }
    if include_jobs {
        print_doc("JOBS.md", JOBS_DOC);
    }
}
