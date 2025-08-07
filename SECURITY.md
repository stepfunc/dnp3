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

## Supply Chain Security

We take supply chain security seriously and have implemented the following measures:

### Automated Vulnerability Scanning

- **Nightly audits**: Dependencies are automatically scanned daily at 2 AM UTC using `cargo audit`
- **Pull request checks**: All PRs are checked for vulnerabilities before merging
- **Immediate notifications**: Maintainers receive automated alerts when vulnerabilities are detected
- **GitHub Security tab integration**: Vulnerability information is surfaced in the repository's Security tab

### Dependency Management

- **Minimal dependencies**: We strive to minimize external dependencies where practical
- **Regular updates**: Dependencies are regularly reviewed and updated to address known vulnerabilities
- **Lock file commits**: `Cargo.lock` is committed to ensure reproducible builds
- **Audit before release**: All releases undergo security audit as part of the CI/CD pipeline

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
