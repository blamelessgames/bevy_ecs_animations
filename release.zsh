#!/usr/bin/env zsh

set -e

# Always run from the project root.
cd "$(dirname "$0")"

RED=$'\033[31m'
RESET=$'\033[0m'

fail() {
    print -u2 -- "${RED}ERROR: $1${RESET}"
    exit 1
}

# Require a clean working tree so we don't accidentally include
# unrelated changes in the release.
if [[ -n "$(git status --porcelain)" ]]; then
    fail "Working tree is not clean. Commit or stash your changes first."
fi

MAIN_TOML="Cargo.toml"

# Extract the main crate's package version.
VERSION=$(awk '
    /^\[workspace.package\]/ { in_package=1; next }
    /^\[/ { in_package=0 }
    in_package && /^version[[:space:]]*=/ {
        gsub(/"/, "", $3)
        print $3
        exit
    }
' "$MAIN_TOML")

[[ -n "$VERSION" ]] || fail "Could not determine package version."

# Remove the development suffix.
[[ "$VERSION" == *-dev ]] || fail "Version '$VERSION' does not end in -dev."

VERSION="${VERSION%-dev}"

print "Preparing release $VERSION"

BRANCH="Release-$VERSION"
TAG="v$VERSION"

# Run the project's full validation suite.
print "Running all.sh..."

if ! ./all.sh; then
    fail "Validation failed. Release aborted; no commit or tag created."
fi

# Avoid overwriting an existing release.
if git show-ref --verify --quiet "refs/heads/$BRANCH"; then
    fail "Branch $BRANCH already exists."
fi

if git show-ref --verify --quiet "refs/tags/$TAG"; then
    fail "Tag $TAG already exists."
fi

# Update the version for the workspace
perl -pi -e "s/\\Q$VERSION-dev\\E/$VERSION/g" "$MAIN_TOML"


# Create the release branch.
git switch -c "$BRANCH"

# Extract the corresponding changelog section.
COMMIT_MSG=$(awk -v heading="### $VERSION" '
    $0 == heading { found=1 }
    found && /^### / && $0 != heading { exit }
    found { print }
' CHANGELOG.md)

if [[ -z "$COMMIT_MSG" ]]; then
    print "No changelog entry found for $VERSION; using fallback message."
    COMMIT_MSG="Release $VERSION"
fi

# Commit the release changes.
TEMP_MSG=$(mktemp)
trap 'rm -f "$TEMP_MSG"' EXIT

print -r -- "$COMMIT_MSG" > "$TEMP_MSG"

git add "$MAIN_TOML"
git commit -F "$TEMP_MSG"

# Create the annotated release tag.
git tag -a "$TAG" -m "Release $VERSION"
git switch -
git branch -D "$BRANCH"

print ""
print "Release preparation complete!"
print "  Tag:    $TAG"
