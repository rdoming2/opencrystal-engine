use engine::party::PartyState;
use engine::rules::Ruleset;

pub struct JobOption {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub sort_order: Option<i32>,
}

pub fn run_party_create_flow(
    session: &mut tui::session::TuiSession,
    runtime: &mut engine::runtime::GameRuntime,
    rules: &Ruleset,
    bindings: &tui::input::InputBindings,
) -> std::io::Result<()> {
    let max_len = rules.party_create.name_length;
    let jobs_enabled = rules.systems.get("jobs").copied().unwrap_or(false);
    let job_options = if jobs_enabled {
        build_available_jobs(runtime)
    } else {
        Vec::new()
    };
    if jobs_enabled && job_options.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no jobs available for party creation",
        ));
    }
    let default_job_index = job_options
        .iter()
        .position(|job| job.is_default)
        .unwrap_or(0);
    let mut members = Vec::new();
    for index in 0..rules.party_size {
        let default_name = format!("Hero {}", index + 1);
        let prompt = format!("Name character {}:", index + 1);
        let name = match tui::dialog::prompt_text(
            session,
            "Create Party",
            &prompt,
            &default_name,
            max_len,
        )? {
            Some(name) => name,
            None => return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit")),
        };
        let job_id = if jobs_enabled {
            let labels = job_options
                .iter()
                .map(|job| job.name.clone())
                .collect::<Vec<_>>();
            match tui::dialog::prompt_choice(
                session,
                bindings,
                "Choose Job",
                "Select a job:",
                &labels,
                default_job_index,
            )? {
                Some(selection) => job_options[selection].id.clone(),
                None => return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit")),
            }
        } else {
            rules.party_create.default_job.clone()
        };
        members.push((name, job_id));
    }
    runtime.party = PartyState::from_created(&runtime.content, rules, members);
    Ok(())
}

pub fn default_party_names(rules: &Ruleset) -> Vec<(String, String)> {
    let max_len = rules.party_create.name_length;
    let default_job = rules.party_create.default_job.clone();
    (1..=rules.party_size)
        .map(|index| {
            let name = format!("Hero {}", index);
            let name = name.chars().take(max_len).collect::<String>();
            (name, default_job.clone())
        })
        .collect()
}

pub fn build_available_jobs(runtime: &engine::runtime::GameRuntime) -> Vec<JobOption> {
    let mut jobs = runtime
        .content
        .jobs
        .jobs
        .iter()
        .filter(|job| job_unlock_available(runtime, job))
        .map(|job| JobOption {
            id: job.id.clone(),
            name: job.name.clone(),
            is_default: job.is_default,
            sort_order: job.sort_order,
        })
        .collect::<Vec<_>>();
    jobs.sort_by(|left, right| {
        let left_order = left.sort_order.unwrap_or(0);
        let right_order = right.sort_order.unwrap_or(0);
        left_order
            .cmp(&right_order)
            .then_with(|| left.name.cmp(&right.name))
    });
    jobs
}

fn job_unlock_available(
    runtime: &engine::runtime::GameRuntime,
    job: &engine::entities::JobDefinition,
) -> bool {
    match job.unlock_flag.as_deref() {
        Some(flag) if !flag.trim().is_empty() => runtime.has_flag(flag),
        _ => true,
    }
}
