#!/usr/bin/env bash
# Install wsx and its shell/agent integration onto this machine.
#
#   ./install.sh              symlink into place (edits here take effect live)
#   ./install.sh --copy       install copies instead of symlinks
#   ./install.sh --uninstall  remove everything this script installs
#   ./install.sh --dry-run    show what would happen
#
# Symlinks are the default deliberately: this repo exists so the tool can be
# iterated on, and a symlinked install means a change here is live immediately
# with no reinstall step.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE=link
DRY=0

for arg in "$@"; do
	case "$arg" in
	--copy) MODE=copy ;;
	--uninstall) MODE=uninstall ;;
	-n | --dry-run) DRY=1 ;;
	-h | --help)
		sed -n '2,9p' "${BASH_SOURCE[0]}" | sed 's/^# \?//'
		exit 0
		;;
	*)
		echo "unknown argument: $arg" >&2
		exit 2
		;;
	esac
done

BIN="$HOME/.local/bin"
FISH_CONF="${XDG_CONFIG_HOME:-$HOME/.config}/fish"
HUSKY_INIT="${XDG_CONFIG_HOME:-$HOME/.config}/husky/init.sh"
COPILOT_HOOK="$HOME/.copilot/hooks/wsx-session.json"
CLAUDE_SETTINGS="$HOME/.claude/settings.local.json"

say() { printf '  %-9s %s\n' "$1" "$2"; }

# link_or_copy <source> <destination>
link_or_copy() {
	local src="$1" dst="$2"
	if [[ "$DRY" == 1 ]]; then
		say "$MODE" "$dst"
		return
	fi
	mkdir -p "$(dirname "$dst")"
	rm -rf "$dst"
	if [[ "$MODE" == link ]]; then
		ln -s "$src" "$dst"
	else
		cp "$src" "$dst"
	fi
	say "$MODE" "$dst"
}

remove() {
	local dst="$1"
	[[ -e "$dst" || -L "$dst" ]] || return 0
	if [[ "$DRY" == 1 ]]; then
		say removed "$dst"
		return
	fi
	rm -rf "$dst"
	say removed "$dst"
}

# --------------------------------------------------------------------------

if [[ "$MODE" == uninstall ]]; then
	echo "Uninstalling wsx"
	remove "$BIN/wsx"
	remove "$FISH_CONF/completions/wsx.fish"
	remove "$FISH_CONF/functions/git.fish"
	remove "$COPILOT_HOOK"
	echo
	echo "Left in place (edit by hand -- they may contain unrelated entries):"
	echo "  $HUSKY_INIT"
	echo "  $CLAUDE_SETTINGS"
	exit 0
fi

echo "Installing wsx from $REPO"
echo

echo "Executable"
link_or_copy "$REPO/bin/wsx" "$BIN/wsx"

if [[ ":$PATH:" != *":$BIN:"* ]]; then
	echo
	echo "  WARNING: $BIN is not on your PATH -- add it to your shell profile."
fi

echo
echo "Fish integration"
if [[ -d "$FISH_CONF" ]]; then
	link_or_copy "$REPO/shell/fish/completions/wsx.fish" "$FISH_CONF/completions/wsx.fish"
	link_or_copy "$REPO/shell/fish/functions/git.fish" "$FISH_CONF/functions/git.fish"
else
	say skipped "no fish config at $FISH_CONF"
fi

echo
echo "Git hooks (husky personal init)"
if [[ -f "$HUSKY_INIT" ]] && grep -qE 'wsx["'"'"']?[[:space:]]+hook' "$HUSKY_INIT"; then
	say ok "$HUSKY_INIT already calls wsx"
elif [[ -f "$HUSKY_INIT" ]]; then
	if [[ "$DRY" == 1 ]]; then
		say append "$HUSKY_INIT"
	else
		cp "$HUSKY_INIT" "$HUSKY_INIT.pre-wsx.bak"
		# Skip the shebang; the rest is appended to whatever is already there.
		tail -n +2 "$REPO/integrations/husky-init.sh" >>"$HUSKY_INIT"
		say append "$HUSKY_INIT (backup: $HUSKY_INIT.pre-wsx.bak)"
	fi
else
	link_or_copy "$REPO/integrations/husky-init.sh" "$HUSKY_INIT"
	[[ "$DRY" == 1 ]] || chmod +x "$HUSKY_INIT"
fi

echo
echo "Agent session hooks"
if [[ -d "$HOME/.copilot" ]]; then
	link_or_copy "$REPO/integrations/copilot/wsx-session.json" "$COPILOT_HOOK"
else
	say skipped "no ~/.copilot"
fi

if [[ -d "$HOME/.claude" ]]; then
	if [[ "$DRY" == 1 ]]; then
		say merge "$CLAUDE_SETTINGS"
	else
		python3 - "$CLAUDE_SETTINGS" "$REPO/integrations/claude/settings-snippet.json" <<'PY'
import json, sys, pathlib

target = pathlib.Path(sys.argv[1])
snippet = json.loads(pathlib.Path(sys.argv[2]).read_text())["hooks"]

data = {}
if target.is_file():
    try:
        data = json.loads(target.read_text())
    except json.JSONDecodeError:
        print(f"  skipped   {target} is not valid JSON -- merge by hand")
        raise SystemExit(0)

hooks = data.setdefault("hooks", {})
added = 0
for event, entries in snippet.items():
    existing = hooks.setdefault(event, [])
    if any("wsx" in json.dumps(e) for e in existing):
        continue
    existing.extend(entries)
    added += 1

if added:
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(data, indent=2) + "\n")
    print(f"  merged    {target}")
else:
    print(f"  ok        {target} already has wsx hooks")
PY
	fi
else
	say skipped "no ~/.claude"
fi

echo
echo "Done. Next steps:"
echo "  1. Put a .wsx.conf on your repo's personal-workspace branch"
echo "     (see $REPO/examples/wsx.conf)"
echo "  2. Run 'wsx setup' in each existing dev worktree, or"
echo "     'wsx new <branch>' to create one that is correct from the start"
echo "  3. Verify with 'wsx doctor'"
