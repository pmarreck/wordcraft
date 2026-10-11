# shellcheck shell=bash disable=SC2034
# Shared implementation for the Nix entrypoints; compatible with macOS Bash 3.2.
WORDCRAFT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd) || exit 1
WORDCRAFT_VERSION=$(awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version = /{gsub(/"/, "", $3);print $3;exit}' "$WORDCRAFT_ROOT/Cargo.toml")

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
nix_cmd() { nix --extra-experimental-features 'nix-command flakes' "$@"; }

# Normalize the executing environment; WSL intentionally selects Linux.
host_platform() {
	local os arch
	os=$(uname -s) || return 1
	arch=$(uname -m) || return 1
	case "$os" in Linux) os=linux ;; Darwin) os=macos ;; *) die "unsupported Nix host: $os" ;; esac
	case "$arch" in arm64|aarch64) arch=aarch64 ;; amd64|x86_64) arch=x86_64 ;; *) die "unsupported architecture: $arch" ;; esac
	WORDCRAFT_HOST="$arch-$os"
	WORDCRAFT_SYSTEM="$arch-$os"
	[[ $os != macos ]] || WORDCRAFT_SYSTEM="$arch-darwin"
}

about() {
	printf '%s %s — %s (%s/%s)\n' "$1" "$WORDCRAFT_VERSION" "$2" "$(uname -s)" "$(uname -m)"
}

git_idle() {
	local state state_path
	for state in rebase-merge rebase-apply MERGE_HEAD CHERRY_PICK_HEAD; do
		state_path=$(git -C "$WORDCRAFT_ROOT" rev-parse --path-format=absolute --git-path "$state") || return 1
		[[ ! -e $state_path ]] || die 'finish or abort the existing Git operation before updating'
	done
	git -C "$WORDCRAFT_ROOT" symbolic-ref --quiet HEAD >/dev/null || die 'checkout a branch before updating'
}

upstream_url_allowed() {
	case "$1" in
		git@github.com:storytold/wordcraft.git|https://github.com/storytold/wordcraft|https://github.com/storytold/wordcraft.git) return 0 ;;
		*) return 1 ;;
	esac
}

valid_timestamp() { [[ $1 =~ ^(0|[1-9][0-9]{0,9})$ ]]; }

# A timestamp alone cannot bless a different branch or a moved upstream ref.
upstream_is_fresh() {
	local now=$1 timestamp oid extra current
	[[ -f $WORDCRAFT_SYNC_STATE ]] || return 1
	read -r timestamp oid extra <"$WORDCRAFT_SYNC_STATE" || return 1
	valid_timestamp "$timestamp" || return 1
	[[ -z $extra && $oid =~ ^[0-9a-f]{40,64}$ ]] || return 1
	((now >= timestamp && now - timestamp < 86400)) || return 1
	current=$(git -C "$WORDCRAFT_ROOT" rev-parse --verify refs/remotes/upstream/main 2>/dev/null) || return 1
	[[ $current == "$oid" ]] || return 1
	git -C "$WORDCRAFT_ROOT" merge-base --is-ancestor "$oid" HEAD
}

# Rebase only a clean, attached branch. Never stash, discard edits, or hide a conflict.
update_upstream() {
	local url timestamp pending
	[[ -z $(git -C "$WORDCRAFT_ROOT" status --porcelain) ]] || die 'working tree is dirty; commit/stash changes or use --no-update'
	if url=$(git -C "$WORDCRAFT_ROOT" config --get remote.upstream.url 2>/dev/null); then
		upstream_url_allowed "$url" || die 'upstream must refer to storytold/wordcraft'
	else
		git -C "$WORDCRAFT_ROOT" remote add upstream https://github.com/storytold/wordcraft.git || return 1
	fi
	printf 'Fetching and rebasing onto upstream/main…\n' >&2
	git -C "$WORDCRAFT_ROOT" fetch upstream main || return 1
	git -C "$WORDCRAFT_ROOT" rebase upstream/main || {
		printf 'Rebase stopped. Resolve with git rebase --continue, or restore with git rebase --abort. No build was published.\n' >&2
		return 1
	}
	timestamp=$(date +%s) || return 1
	valid_timestamp "$timestamp" || return 1
	pending="$WORDCRAFT_SYNC_STATE.new.$$"
	printf '%s %s\n' "$timestamp" "$(git -C "$WORDCRAFT_ROOT" rev-parse upstream/main)" >"$pending" || return 1
	command mv -f "$pending" "$WORDCRAFT_SYNC_STATE"
}

# auto = daily refresh; force = explicit --update; off = cached sync only.
ensure_upstream() {
	local mode=$1 now common url
	git_idle
	common=$(git -C "$WORDCRAFT_ROOT" rev-parse --path-format=absolute --git-common-dir) || return 1
	WORDCRAFT_SYNC_STATE="$common/wordcraft-upstream-sync"
	now=$(date +%s) || return 1
	valid_timestamp "$now" || die 'cannot read a valid current timestamp'
	if url=$(git -C "$WORDCRAFT_ROOT" config --get remote.upstream.url 2>/dev/null); then
		upstream_url_allowed "$url" || die 'upstream must refer to storytold/wordcraft'
	fi
	if [[ $mode != force ]] && upstream_is_fresh "$now"; then return 0; fi
	[[ $mode != off ]] || die 'upstream sync is missing, stale (24 hours), or not in this branch; run ./build --update on a clean checkout'
	mkdir "$WORDCRAFT_SYNC_STATE.lock" 2>/dev/null || die 'another build is updating upstream'
	# Scope cleanup to the update, including errors and ordinary interruption.
	(
		trap 'rmdir "$WORDCRAFT_SYNC_STATE.lock"' EXIT
		trap 'exit 130' INT
		trap 'exit 143' TERM
		update_upstream
	)
}

target_info() {
	case "$1" in
		aarch64-macos) WORDCRAFT_TRIPLE=aarch64-apple-darwin; WORDCRAFT_OS=macos; WORDCRAFT_ARCH=aarch64 ;;
		aarch64-linux) WORDCRAFT_TRIPLE=aarch64-unknown-linux-gnu; WORDCRAFT_OS=linux; WORDCRAFT_ARCH=aarch64 ;;
		x86_64-linux) WORDCRAFT_TRIPLE=x86_64-unknown-linux-gnu; WORDCRAFT_OS=linux; WORDCRAFT_ARCH=x86_64 ;;
		aarch64-windows) WORDCRAFT_TRIPLE=aarch64-pc-windows-gnullvm; WORDCRAFT_OS=windows; WORDCRAFT_ARCH=aarch64 ;;
		x86_64-windows) WORDCRAFT_TRIPLE=x86_64-pc-windows-gnu; WORDCRAFT_OS=windows; WORDCRAFT_ARCH=x86_64 ;;
		*) die "unsupported target: $1" ;;
	esac
}

# Publish one directory link only after Nix succeeds. Retain a GC root per target/profile.
build_target() {
	local target=$1 profile=$2 attribute link output suffix destination pending
	target_info "$target"
	attribute="packages.$WORDCRAFT_SYSTEM.$target"
	[[ $profile != debug ]] || attribute="$attribute-debug"
	link="$WORDCRAFT_ROOT/.nix-out/$WORDCRAFT_TRIPLE/$profile"
	mkdir -p "$(dirname "$link")" || return 1
	nix_cmd build "$WORDCRAFT_ROOT#$attribute" --out-link "$link" --print-build-logs || return 1
	output=$(cd "$link" && pwd -P) || return 1
	suffix=
	[[ $WORDCRAFT_OS != windows ]] || suffix=.exe
	[[ -x "$output/bin/wordcraft$suffix" && -x "$output/bin/wordcraft-cli$suffix" ]] || die "missing executables in $output/bin"
	destination="$WORDCRAFT_ROOT/bin/$WORDCRAFT_OS/$WORDCRAFT_ARCH"
	mkdir -p "$(dirname "$destination")" || return 1
	[[ ! -e $destination || -L $destination ]] || die "refusing to replace existing directory: $destination"
	pending="$destination.new.$$"
	ln -s "../../.nix-out/$WORDCRAFT_TRIPLE/$profile/bin" "$pending" || return 1
	# Never follow a directory symlink. Rename over the old selector atomically,
	# using the host's spelling for that operation.
	if ! mkdir "$destination.lock" 2>/dev/null; then
		rm -f -- "$pending"
		die "another build is publishing $destination"
	fi
	local moved=0
	if [[ $(uname -s) == Darwin ]]; then
		command mv -fh "$pending" "$destination" && moved=1
	else
		command mv -Tf "$pending" "$destination" && moved=1
	fi
	if ((moved)); then
		rmdir "$destination.lock"
	else
		rmdir "$destination.lock"
		return 1
	fi
	printf 'Built %s (%s): %s/wordcraft%s\n' "$target" "$profile" "$destination" "$suffix" >&2
}
