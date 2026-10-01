# Repository Guidelines

## Repository Scope

- Jcode Desktop is in a separate repository.

## Branding Boundaries

This is a fork of upstream `1jehuang/jcode`. The user-visible name is **Kraivcode**
(`nkswalih/kraivcode`). When renaming, sort every hit into one of three buckets:

- **Fork-owned - rename it.** User-facing product strings: CLI `name`/`about`,
  ACP `agentInfo.name`/`title`, process titles (`kraivcode:s:`, `kraivcode:c:`,
  `kraivcode:d:`, `kraivcode:client`, `kraivcode:selfdev`), the macOS
  notification broker bundle, notification titles and bodies, and the release
  pipeline (repo URLs, Homebrew tap, AUR package).
- **Upstream-hosted - leave it.** `jcode.sh`, `api.jcode.sh`,
  `telemetry.jcode.sh`, `jcode.sh/pricing`, `jcode.sh/account`, the `jcode`
  provider and subscription id, and `Jcode Account` copy. The fork still calls
  these services; renaming the label would make it claim to host them.
- **Structural - out of scope.** `~/.jcode/`, `JCODE_*` env vars, the
  `jcode-*` crate names, the `jcode://` URL scheme, the `jcode` provider id,
  `com.jcode.*` bundle identifiers, and the installed command name `jcode`
  (`binary_stem()` in `crates/jcode-build-support/src/paths.rs`). These are the
  engine's data and wire layout, not the brand.

`LICENSE` keeps its MIT upstream attribution, and the README credits the upstream
project by name. Do not rewrite `changelog/**`: it is a historical record.

### Project-local skills directory

This repository ships a bundled skill at `.kraivcode/skills/optimization/`. The
loader in `crates/jcode-base/src/skill.rs` scans four project-local
conventions: `.kraivcode`, `.jcode`, `.agents`, and `.claude`. All four are
supported on purpose. `.jcode` stays because the same loader runs in every user
project, not just this repository; dropping it would silently stop loading
skills that other users already have on disk. Add new conventions to the list
rather than replacing it.

### Pruned upstream orphans

These were deleted intentionally. Upstream may still carry them; do not restore
them during a merge.

- `indian-man-portrait.svg` - a 1024x1024 gradient portrait card from upstream
  `04dedf176`, referenced by nothing.
- `score_shard.py` - upstream `3af1b5442`, a one-off research script with
  hardcoded `/home/jeremy/...` input paths. It cannot run anywhere but its
  author's machine.

## Development Workflow

- **Use the user's Git identity** - Create commits with the configured
  `user.name` and `user.email`. Do not override them with `Jcode`, `Jcode agent`,
  or a fabricated agent email. Preserve existing contributor attribution when
  integrating work. If no identity is configured, ask rather than inventing one.
- **Welcome pull requests from everyone** - Review contributions on their merits,
  regardless of whether the author is a maintainer, an existing contributor, a
  first-time contributor, or an agent. Good PRs can be merged directly after review
  and validation. Do not require a maintainer-authored rewrite merely because of
  who submitted the change. See `CONTRIBUTING.md` for the contribution policy.
- **Keep work scoped** - Work on your own branch and preserve unrelated work. When
  the user asks you to review or integrate a PR or branch, you may inspect, test,
  and integrate that contribution regardless of author status. Do not pull in
  unrelated branches or merge a PR without user authorization.

## Install Notes
- `~/.local/bin/jcode` is the launcher symlink used from `PATH`.
- `~/.jcode/builds/current/jcode` is the active local/source-build channel; self-dev builds and `scripts/install_release.sh` point the launcher here.
- `~/.jcode/builds/stable/jcode` is the stable release channel; `scripts/install.sh` installs this and points the launcher here.
- `~/.jcode/builds/versions/<version>/jcode` stores immutable binaries.
- `~/.jcode/builds/canary/jcode` still exists for canary/testing flows, but it is not the primary self-dev install path.
- On Windows, the equivalents are `%LOCALAPPDATA%\\jcode\\bin\\jcode.exe` for the launcher, `%LOCALAPPDATA%\\jcode\\builds\\stable\\jcode.exe` for stable, and `%LOCALAPPDATA%\\jcode\\builds\\versions\\<version>\\jcode.exe` for immutable installs; `scripts/install.ps1` currently installs the stable channel.
- Ensure `~/.local/bin` is **before** `~/.cargo/bin` in `PATH`.

## Verifying a change at runtime

`cargo build` alone proves nothing about behavior. `jcode run` and interactive
sessions are served by the long-lived daemon at
`~/.jcode/builds/shared-server/jcode`, which is a symlink into
`~/.jcode/builds/versions/<version>/`. Until that symlink is repointed and the
daemon restarted (`jcode self-dev --build`), a freshly built binary is inert and
every runtime check silently measures the old code.

To test a change without disturbing the shared daemon or the caller's session,
run your build against its own socket:

```bash
cargo build --profile selfdev
./target/selfdev/jcode run --no-update --socket /run/user/1000/jcode-mytest.sock '<prompt>'
```

Two things that waste time otherwise:

- `crate::logging::info` writes to a log file, not stderr, so instrumenting a
  code path with it produces no visible output under `--trace`. Use `eprintln!`
  for throwaway diagnostics and delete it before committing.
- Confirm which binary you are actually inspecting. `strings` on
  `builds/shared-server/jcode` reads a 70-byte symlink, not a program; resolve it
  with `readlink -f` first.
