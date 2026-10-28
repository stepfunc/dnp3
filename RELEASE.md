# Release Process

Publishing is handled by `.github/workflows/ci.yml`: **pushing a tag `X.Y.Z`
(or a milestone `X.Y.Z-Mn` or release candidate `X.Y.Z-RCn`) to a commit on `main` publishes that version** to crates.io,
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

Open the PR with the `conformance` label (`gh pr create --label conformance`).
The conformance tests don't run on unlabeled PRs, and a failure on the tag
blocks every publisher, so merge the PR to `main` only once both conformance
jobs are green on its latest revision, after its final update with `main`.

## 2. Tag on `main` and push

CI only publishes from tags, and by convention tags live on `main`. Tag the
commit the `prepare` PR's merge produced, by its SHA, since `main` may have
moved since:

```bash
git checkout main && git pull
SHA=$(gh pr view <prepare PR number> --json mergeCommit --jq .mergeCommit.oid)
git tag X.Y.Z "$SHA" && git push origin X.Y.Z
```

To tag a later commit instead, run the conformance tests on it first
(`gh workflow run conformance.yml --ref main`), wait for the run to pass, and
tag the SHA it tested (its `headSha` in `gh run list --workflow conformance.yml
--json headSha,conclusion`).

crates.io publishing is **irreversible** — a version can never be re-published.
Confirm the version before pushing. (`-Mn` and `-RCn` publish as pre-releases, which
cargo won't select by default.)

## 3. After CI completes

Review and publish the **draft** GitHub release CI created. The publishers are
idempotent, so re-running the tag's workflow safely retries any that failed.

## Dependencies

Dependency security is handled **continuously**, not at release time:
`.github/workflows/security-audit.yml` runs `cargo audit` and `osv-scanner` daily,
on every PR, and on every `main` push that touches `Cargo.toml`/`Cargo.lock`, and
fails on advisories.
Security patches land as their own PRs as advisories appear — they are not
batched onto releases.

The release's dependency responsibilities are therefore narrow:

- **Gate, don't churn:** the security audit must be green on the `prepare` PR
  (it runs on every PR). Don't tag if it's
  red. The prebuilt bindings (C/.NET/Java) freeze whatever is in `Cargo.lock` at
  tag time, so a clean audit at the tag is what protects binding consumers —
  pure-Rust crate consumers re-resolve on their own.
- **Freshen early, once per cycle:** do the routine "update dependencies" pass
  at the *first* RC/milestone of a new minor version, not at each RC and not at
  the final release. Late-cycle dependency churn undermines the stabilization
  the RC process exists to provide. Subsequent RCs and the final release take
  **security patches only** (targeted `cargo update -p <crate>`).

## CHANGELOG conventions

- The upcoming release has one section, headed by its final version (`### X.Y.Z ###`), from the
  first milestone through the final release. Milestone and release-candidate tags publish it as it
  stands; there are no per-pre-release sections to collapse.
- Markers: `:rotating_light:` for a vulnerability with an advisory (the first bullet, led by bold
  "Security advisory:"), then optionally one bold headline bullet for the release's theme, then by
  type: `:bell:` behavior or compatibility changes, `:shield:` hardening, `:star:` features,
  `:bug:` fixes, `:wrench:` tooling and CI, `:book:` docs. Each bullet links its PR or issue.
