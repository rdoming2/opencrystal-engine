# Job System

OpenCrystal ships with a modular job system so games can pick the
progression model that fits their vision. Every actor has one primary job
and, if the rules allow it, an optional secondary job. As actors gain
experience, jobs unlock new base stats, spells, and abilities according to
per-job growth data defined in `content/<pack>/entities/jobs.json`.

## Related docs

- `README.md` for project overview and CLI basics.
- `SCHEMAS.md` for job schema details.
- `CONTENT_AUTHORING_GUIDE.md` for content pack workflow.

Use `cryst build docs -j` to print this file to stdout.

## Progression Modes

The progression mode is configured under `rules.json -> progression_mode`. Four
modes are available right now:

- `character`: (default) a single `Actor.level`/`Actor.exp` pair drives
  progression. Switching jobs keeps the current level and the job simply
  reapplies its stat growth for that level.
- `job`: experience is tracked per job. `job_system.job_exp_curve` defines
  the thresholds for each job. When a job change happens the actor's base
  stats are regenerated from that job's growth table/formula at the saved
  level.
- `job_points`: characters still earn global EXP via `rules.exp_curve`,
  but each job also accumulates JP (job points), which can be spent via
  the Job menu to unlock additional spells or abilities. Jobs may still
  auto-unlock stat/spell gains by level while JP purchases provide finer
  control.
- `activity`: actors grow by use-based weapon/magic proficiencies (0.0-1.0)
  with soft caps. EXP/level-ups are disabled, but jobs remain as baseline
  templates for stats, equipment slots, and unlock lists.

JP gains, secondary job availability, and the job curve are all controlled
from the same `job_system` block. JP can be configured via `job_system.jp_mode`:
`earn` grants JP that accumulates automatically as jobs level and cannot be
spent; `spend` grants JP that must be spent in the
Job menu to unlock spells or abilities. Changing modes only requires swapping
a single field in `rules.json` and adjusting the job definitions to include
`unlock_level`/`jp_cost` entries for spells and abilities.

## Secondary Jobs

When `job_system.secondary_jobs` is enabled, the Job menu exposes a slot
for assigning a secondary job. Secondary jobs share spells, abilities, and
JP unlock state with the actor, but they do not change the actor's base
stats or equipment slots.

## Job Menu Flow

- The main menu's Jobs entry lists all defined jobs and highlights the
  currently active actor.
- Confirming a job (Enter) assigns it as the primary job. The TUI warns
  which job is currently equipped and which job will become primary.
- When JP is enabled, the Job panel also surfaces the current JP balance
  for each job and the unlockable spells/abilities that still have a cost.
- Secondary job assignment is an optional action (mapped to Pause/"Mag" on
  the menu). It only appears if the rules allow it.
- The Jobs menu is hidden automatically when `progression_mode` is `activity`.

## Data and Configuration

- Job definitions may include `jp_cost` and `unlock_level` helpers for both
  spells and abilities. These fields drive whether a feature unlocks
  automatically or must be purchased.
- The `Job` data also exposes `magic_schools` and `description` fields that
  the job menu uses to show job flavor and school coverage.
- The engine keeps a `job_progress` map per actor so save files can persist
  job-specific levels, JP balances, and the learned ability set.
- When using `progression_mode: activity`, tune weapon/magic gains and rank
  labels under `rules.json -> activity_progression`.

By following these conventions the job system remains data-driven and
flexible: designers can keep the classic single-progress model or switch to
per-job JP-heavy builds without touching rendering or battle logic.
