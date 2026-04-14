#!/usr/bin/env bash
set -euo pipefail

VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
MAJOR=$(echo "$VERSION" | cut -d. -f1)
MINOR=$(echo "$VERSION" | cut -d. -f2)
PATCH=$(echo "$VERSION" | cut -d. -f3)
NEW_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"

sed -i "s/^version = \"$VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
echo "Bumped version $VERSION -> $NEW_VERSION"
echo "NEW_VERSION=$NEW_VERSION" >> bump.env

git add Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION"
git push origin dev

git fetch origin pre-prod || git push origin dev:pre-prod
git checkout pre-prod
git merge dev --no-ff -m "chore: merge dev into pre-prod - release $NEW_VERSION"
git push origin pre-prod
