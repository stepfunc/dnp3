# Security Policy

Thank you for helping keep the `dnp3` library secure.

This library is distributed under a non-commercial license but otherwise follows the norms and expectations of
open-source software. We support coordinated vulnerability disclosure using GitHub’s built-in tooling.

## Reporting a Vulnerability

Please do **not** open a public issue for potential security problems.

Instead, go to the repository’s **Security** tab and click **“Report a vulnerability”** to submit a private report to
the maintainers.

> If the “Report a vulnerability” button is not visible, make sure you’re signed in to GitHub.

No PGP or email is required.

## What to Expect

For reports submitted through GitHub’s private vulnerability reporting:

- We aim to **acknowledge reports within 48 hours**
- We aim to provide **initial triage feedback within 5 business days**
- We typically resolve confirmed issues within **90 days** (often sooner)

These are non-binding response targets based on community norms. They do **not** constitute a service-level agreement.

For commercial users, all formal response obligations are governed solely by the terms of your signed support agreement.

## Staying Informed

To stay up to date:

- **Watch this repository**: Click "Watch" → "Custom" → enable "Releases" and "Security alerts"
- Use the
  [GitHub Security Advisories RSS feed](https://github.blog/changelog/2022-10-25-syndicate-github-security-advisories-with-rss/)
- Use tools like `cargo audit` or your organization's OSS scanners

## How the Library Is Hardened

The [security page](https://docs.stepfunc.io/dnp3/1.8.0/guide/docs/security) of the guide describes the engineering measures: memory safety, the lints that restrict
sources of panics, the link-time proof that the parsers can't panic, protocol validation and the TLS trust model.

## Supply Chain Security

We take supply chain security seriously and have implemented the following measures:

### Automated Vulnerability Scanning

- **Nightly audits**: Dependencies are automatically scanned daily at 2 AM UTC using `cargo audit` and `osv-scanner`, which also
  covers advisories published on GitHub that don't reach the RustSec database
- **Pull request checks**: All PRs are checked for vulnerabilities before merging
- **Notifications**: A failed scheduled scan notifies maintainers through GitHub Actions
- **Reports**: Each scan's report is kept as a workflow artifact, and a failed `cargo audit` lists the advisories in the job summary

### Dependency Management

- **Minimal dependencies**: We strive to minimize external dependencies where practical
- **Regular updates**: Dependencies are regularly reviewed and updated to address known vulnerabilities
- **Lock file commits**: `Cargo.lock` is committed to ensure reproducible builds
- **Audit before release**: A release is only tagged once the security scans pass on its release PR

### Build and Release

- **Pinned build tools**: CI pins every GitHub Action to a commit, the Rust toolchain to a version, and its other
  tools to fixed versions, commits or checksums. The workflows are checked with `zizmor` on every change
- **Approved publishing**: Releases publish only from tags, and only after a maintainer approves the run
- **Signed Java artifacts**: The jars on Maven Central carry PGP signatures

### Verifying Release Artifacts

Starting with 1.8.0, each file attached to a GitHub release has a build provenance attestation that records the
workflow run and commit that built it. Verify a downloaded file with the GitHub CLI:

```bash
gh attestation verify dnp3-1.8.0.zip --repo stepfunc/dnp3
```

nuget.org signs each package it accepts, which changes the file, so verify the copy of the NuGet package attached to
the GitHub release. The crate on crates.io is published by the release tag's workflow through crates.io trusted
publishing.

### For Users

To check for vulnerabilities in your deployment:

```bash
# Install cargo-audit
cargo install cargo-audit --locked

# Run audit in your project
cargo audit
```

We recommend integrating `cargo audit` into your CI/CD pipeline for continuous monitoring.

## Commercial Support

Customers with support agreements receive technical help and guidance but must track public releases using the same
tools available to the open-source community.

Optional security notification services may be introduced in the future.

## CRA and Regulatory Alignment

Step Function I/O is preparing for EU Cyber Resilience Act compliance. This library follows a coordinated disclosure
process compatible with Articles 11–15 of Regulation (EU) 2024/2847.

---

If you're unsure whether something is a vulnerability, please report it privately anyway.
