---
title: "Add a skill command that prints or installs the bundled agent skill"
kind: feature
component: cli
origin: request
---
# Add a skill command that prints or installs the bundled agent skill

blendwerk ships an agent skill in `skills/blendwerk/`, but installing it needs the skills CLI (`npx skills add jakobwesthoff/blendwerk`) or a manual copy or symlink, as the README's agent skill section describes. The user wants blendwerk to provide the skill itself through a command, in the same manner as asqr already does, so the installed skill always matches the installed blendwerk. Requested on 2026-10-10.

## Goal

`blendwerk skill` prints the skill, and `blendwerk skill --install <DIR>` writes it into a skills root such as `.claude/skills`, as `<DIR>/blendwerk/...`, and prints what it installed.

## Proposal

Follow asqr's implementation (asqr: `src/cli/skill.rs`, the `Command::Skill { install }` subcommand in `src/cli/mod.rs`, the "skill" section of its README):

- Compile the skill into the binary with `include_str!`, so it cannot drift from the binary. Unlike asqr's single `SKILL.md`, blendwerk's skill has reference files besides `skills/blendwerk/SKILL.md`: `references/cli.md`, `references/mock-structure.md` and `references/request-logs.md`. All four are embedded, and `--install` writes the whole directory tree.
- Without `--install`, print `SKILL.md` to stdout. Decide whether the references are printed too, or only listed.
- Remove `"skills/"` from the `exclude` list in `Cargo.toml`, since the crate then embeds those files; `cargo package` fails to verify otherwise.
- Rewrite the README's agent skill section to lead with `blendwerk skill --install .claude/skills`, keeping the skills CLI as an alternative.
- Add tests for printing and installing, as asqr has (asqr: `tests/cli_skill.rs`), including that a second install replaces an older skill.

## Open questions

- CLI shape: blendwerk has no subcommands today. `Args` in `src/main.rs` takes the mock directory as a positional argument, so a `skill` subcommand needs clap's optional subcommand handling (for example `args_conflicts_with_subcommands`), and a mock directory literally named `skill` would then have to be passed as `./skill`. The alternative is a `--skill` flag; asqr's form is a subcommand.
