# wsx

**Keep your personal files — planning docs, agent instructions, tooling state —
in every git worktree, without ever committing them to a shared branch.**

If you work in git worktrees and keep notes, `AGENTS.md`/`CLAUDE.md` files, or
local tooling state alongside the code, you have hit this problem: those files
are useful in *every* worktree, but they belong in *none* of the branches. The
usual workarounds all have a sting:

| Workaround | What goes wrong |
|---|---|
| Commit them to the feature branch | They leak into PRs, and every branch drifts apart |
| Keep them untracked | `git status` is permanently noisy; `git add -A` sweeps them in |
| Symlink into each worktree | Tools that don't follow symlinks break; one careless write hits every worktree at once |
| Copy by hand | Two worktrees diverge silently, and you lose whichever you forget |

wsx keeps one authoritative copy on a private `personal-workspace` branch and
gives every other worktree a **real file copy**, synced both ways, ignored by
git, and blocked from being committed anywhere else.

```
personal-workspace worktree            dev worktree (feature/ABC-123)
  .planning/         <--- sync --->      .planning/        (ignored + guarded)
  AGENTS.md          <--- sync --->      AGENTS.md         (ignored + guarded)
  .agents/skills/    ---- pull --->      .agents/skills/   (mirror, read-only)
  .state/db.sqlite   <--- lww ---->      .state/db.sqlite  (newest wins, backup kept)
         ^
         └── the only branch that commits these paths
```

---

## Quick start

```sh
git clone https://github.com/denniyahh/wsx.git ~/dev/personal_projects/wsx
cd ~/dev/personal_projects/wsx && ./install.sh
```

Then, in the repo you want to manage:

```sh
# 1. Create the private branch and a worktree for it, if you don't have one
git worktree add ../myrepo-personal -b personal-workspace

# 2. Describe what to manage
cp ~/dev/personal_projects/wsx/examples/wsx.conf ../myrepo-personal/.wsx.conf
$EDITOR ../myrepo-personal/.wsx.conf

# 3. Move your personal files onto that branch and commit them there
#    (they stay ignored everywhere else from now on)

# 4. Set up any existing dev worktree
wsx setup

# 5. Check it
wsx doctor
```

From then on, new worktrees are one command:

```sh
wsx new feature/ABC-123
```

---

## How it works

### The manifest

A `.wsx.conf` at the repo root **on the `personal-workspace` branch** lists what
to manage. One `<policy> <path>` entry per line; the longest matching entry
wins, so a nested entry overrides its parent.

```conf
sync  .planning
sync  AGENTS.md
lww   .planning/index.sqlite    # nested override: unmergeable file inside a synced tree
pull  .agents                   # vendored reference material
```

| Policy | Direction | Conflict behaviour | Use for |
|---|---|---|---|
| `sync` | both ways | reports, clobbers nothing | text you edit anywhere |
| `pull` | source → worktree | local edits discarded | vendored/read-only content |
| `lww` | both ways | newest mtime wins, loser backed up | SQLite, append-only logs |
| `skip` | — | — | carving a path out |

### Three-way sync

`sync` paths are compared against a **base snapshot** recorded at the last sync
(`<git-dir>/wsx/base.json`), which is what makes real conflict detection
possible rather than "newest file wins":

| Source | Worktree | Result |
|---|---|---|
| changed | unchanged | pull |
| unchanged | changed | push |
| unchanged | unchanged | nothing |
| **changed** | **changed** | **conflict — nothing is overwritten** |

On conflict, wsx writes the source version alongside yours as
`<file>.wsx-remote` so you can diff before choosing:

```sh
wsx status                          # what conflicted
diff .planning/NOTES.md{,.wsx-remote}
wsx resolve .planning/NOTES.md --mine     # keep this worktree's copy
wsx resolve .planning/NOTES.md --theirs   # take the source copy
```

Deletions propagate too: delete a file in either place and the other side
follows, because the base snapshot distinguishes "deleted here" from "never
existed".

### Why copies, not symlinks

A symlinked `.planning` is one directory with many names. Tools that don't
resolve symlinks (some editors, indexers, and archive utilities) misbehave, and
there is no such thing as "the version in this worktree" — a stray write from
any worktree hits all of them instantly, with no base to detect it against.

Copies give each worktree a genuine independent version, which is what makes
conflict detection meaningful. wsx migrates existing symlinks automatically:
it detects a path reached *through* a symlink at any level, replaces it with a
real file, and materialises symlinked parent directories before writing so it
never writes through the link into the source.

### Ignoring, without hiding it from the source branch

The managed paths must be ignored in dev worktrees but **committable on
`personal-workspace`** — so `.gitignore` and `.git/info/exclude` are both wrong,
since each is shared by every worktree.

wsx uses per-worktree ignore rules instead: `extensions.worktreeConfig` plus
`core.excludesFile` set with `git config --worktree`, pointing at a generated
`.git/info/exclude.wsx-<worktree>`. The source worktree simply never gets one.

> Git supports exactly one `core.excludesFile`, so wsx **inlines your existing
> global ignore file** into the generated one. Without that, setting it would
> silently drop every global rule you had.

Orphaned exclude files are pruned automatically when their worktree disappears.

### The guard

Ignore rules stop accidents; they don't stop `git add -f`. A `pre-commit` and
`pre-push` guard blocks managed paths on any branch except
`personal-workspace`:

```
wsx: blocked -- personal workspace paths staged on branch 'feature/ABC-123':
    .planning/STATE.md
  Only the 'personal-workspace' branch may commit these.
  Unstage them with: git restore --staged .planning/STATE.md
  They are synced to the source worktree by 'wsx push' -- nothing is lost.
```

Everything except the guard **fails open** — a sync problem reports and moves
on, and can never block a commit.

How the guard is wired depends on the repo:

- **husky repos** (a `package.json` at the root): husky sources
  `~/.config/husky/init.sh`, which calls `wsx hook <name>`. `wsx new` runs
  `npm install` to regenerate `.husky/_/`.
- **everything else** (Python, Go, …): `wsx setup`/`wsx new` write small shims
  for `pre-commit`, `pre-push` and `post-merge` into the shared `.git/hooks`.
  They are untracked and live in the common git dir, so one install covers
  every worktree. Each carries a `# wsx-managed git hook shim` marker; a hook
  without it is never overwritten. If `core.hooksPath` is set, wsx leaves hooks
  to that manager — call `wsx hook <name>` from it.

---

## Commands

| Command | What it does |
|---|---|
| `wsx new <branch>` | Create a worktree, arm hooks, copy state in — all of setup in one step |
| `wsx rm [worktree]` | Push state back, *then* remove the worktree |
| `wsx list` | Every worktree and its sync state |
| `wsx status` | What has diverged in this worktree |
| `wsx pull` / `push` / `sync` | Move changes deliberately (`-n` to preview) |
| `wsx resolve <path> --mine\|--theirs` | Settle a conflict |
| `wsx setup` | Prepare an existing worktree |
| `wsx doctor` | Verify ignores, guards and hooks are active |
| `wsx guard [--range A..B]` | The commit/push check (used by hooks) |
| `wsx hook <name>` | Hook entry point (used by git and agent hooks) |

### `wsx rm` matters

A worktree's copies are the **only** place their edits exist until they are
pushed. `git worktree remove` therefore destroys unpushed personal work
silently. `wsx rm` pushes first, and refuses outright on unresolved conflicts or
uncommitted tracked changes unless given `--force`.

The fish integration routes `git worktree remove` through `wsx rm` for this
reason.

---

## Automation

Once installed, most syncing happens without you:

| Trigger | Action |
|---|---|
| `pre-commit` | guard, then push local edits back |
| `pre-push` | guard the outgoing range, then push back |
| `post-merge` | pull |
| agent session start | pull (+ arm the commit guard if missing) |
| agent session end | push |
| any `wsx` command | reinstall missing or stale ignore rules |

That last row is what makes worktrees **self-healing**: one created with a bare
`git worktree add` becomes fully correct on the next wsx run, with no memory
required.

> **Why isn't worktree creation itself hooked?** It can't be, cleanly. Git does
> run `post-checkout` for `git worktree add`, but husky's shim exits before
> loading the personal init file unless the new worktree already contains a
> matching `.husky/<hook>` file — and `.husky/` is usually tracked and
> team-shared, so a personal tool has no business adding one. Self-healing
> closes the same gap without touching anything shared.

### Escape hatch

`WSX_SKIP=1` disables every hook and the fish wrapper for a single command.

---

## Requirements

- git ≥ 2.31 (for `extensions.worktreeConfig` with `git config --worktree`)
- Python ≥ 3.9, standard library only
- Optional: [husky](https://typicode.github.io/husky/) for git hooks in npm
  repos (other repos get plain `.git/hooks` shims), fish for shell integration,
  npm if `wsx new` should arm husky for you

## Testing

```sh
./tests/sandbox.sh
```

Builds a throwaway repo in `$(mktemp -d)` and exercises setup, both sync
directions, conflict detection and both resolutions, deletion propagation,
every policy, the guard, and the source-worktree refusal. It touches nothing
outside its temp directory.

## Layout

```
bin/wsx                     the tool (single file, stdlib only)
install.sh                  symlink/copy into place; --uninstall to reverse
examples/                   annotated manifest template
integrations/               husky init, Copilot hook, Claude settings snippet
shell/fish/                 completions + the git worktree remove guard
tests/sandbox.sh            end-to-end suite
```

## License

MIT — see [LICENSE](LICENSE).
