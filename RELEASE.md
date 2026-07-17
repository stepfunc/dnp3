# Release Process

Publishing is handled by `.github/workflows/ci.yml`: **pushing a tag `X.Y.Z`
(or `X.Y.Z-RCn`) to a commit on `main` publishes that version** to crates.io,
Maven Central, NuGet, and the docs site, and opens a draft GitHub release. This
file covers only the human steps around that trigger.

## 1. Prepare a `prepare X.Y.Z` PR against `main`

Bump the version and refresh `Cargo.lock`:

```bash
./bump-version.sh X.Y.Z
```

The script rewrites the version in every file that hard-codes it (Cargo
manifests, the poms, the .csproj examples, the C `CMakeLists.txt`, and
`guide/sitedata.json`), updates `Cargo.lock`, and verifies nothing was missed.
The file list lives in the script — keep it there, not duplicated here.

Then update `CHANGELOG.md` (see conventions below). If any dependency versions
changed, refresh `allowed.json` so the packaging license check passes, and
verify any dependency/security claims in the changelog against the actual
`Cargo.lock` versions.

Open the PR and merge it to `main`.

## 2. Tag on `main` and push

CI only publishes from tags, and by convention tags live on `main`:

```bash
git checkout main && git pull
git tag X.Y.Z && git push origin X.Y.Z
```

crates.io publishing is **irreversible** — a version can never be re-published.
Confirm the version before pushing. (`-RCn` publishes as a pre-release, which
cargo won't select by default.)

## 3. After CI completes

Review and publish the **draft** GitHub release CI created. The publishers are
idempotent, so re-running the tag's workflow safely retries any that failed.

## CHANGELOG conventions

- Each release candidate gets its own `### X.Y.Z-RCn ###` section while the
  pre-release cycle is ongoing, so adopters can see the delta between RCs.
- **When cutting the final `X.Y.Z`, collapse all `X.Y.Z-RCn` sections into a
  single `### X.Y.Z ###` section** — merge and de-duplicate the bullets and drop
  the per-RC headers. The final release gets one coherent entry.
- Bullet prefixes in use: `:star:` feature, `:shield:` security, `:wrench:`
  tooling/CI, `:book:` docs. Reference the PR/issue number.
