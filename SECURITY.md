# Security Policy

Robonomics takes the security of its software seriously. We appreciate the efforts of
security researchers and the community in helping us keep users safe.

## Supported Versions

Only the most recent major release line receives security fixes. Users are strongly
encouraged to stay up to date with the latest release.

| Version | Supported          |
| ------- | ------------------ |
| 50+     | :white_check_mark: |
| 4.x.x   | :x:                |
| 3.x.x   | :x:                |
| 2.x.x   | :x:                |
| 1.x.x   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues, discussions,
or pull requests.** Publicly disclosing a vulnerability before a fix is available puts
users and the network at risk.

Instead, report vulnerabilities privately using one of the following channels:

1. **[GitHub Security Advisories](https://github.com/airalab/robonomics/security/advisories/new)**
   (preferred) — this keeps the report private to maintainers until a fix is released.
2. **Email** — research@robonomics.network. If the report contains sensitive details,
   ask us for a PGP key before sending.

When reporting, please include as much of the following as possible to help us triage
quickly:

- A description of the vulnerability and its potential impact
- Steps to reproduce, proof-of-concept code, or a minimal test case
- Affected version(s), component(s), or crate(s)
- Any suggested mitigation or fix, if known

### What to Expect

- **Acknowledgement** within 5 business days of your report.
- **Initial assessment** (severity, affected versions) within 10 days.
- **Regular updates** on remediation progress at least every 2 weeks until resolved.
- **Coordinated disclosure** — we will work with you on an agreed disclosure timeline
  (typically 90 days, or sooner once a fix ships) and credit reporters who wish to be
  named in the release notes/advisory.

Please act in good faith: only interact with your own accounts/data or with explicit
permission, avoid privacy violations and service disruption, and give us reasonable time
to remediate before any public disclosure.

## Scope

This policy covers the code in this repository (runtime, pallets, node, and related
tooling) and its direct dependencies as pinned in `Cargo.lock`. Vulnerabilities in
unrelated third-party services or infrastructure are out of scope; please report those to
their respective maintainers.

## Zero Vulnerability Tolerance

We maintain a **zero vulnerability tolerance policy** for all releases. A new release
**cannot be published** while any known vulnerability affecting the codebase or its
dependencies remains unresolved.

This means:

- All known vulnerabilities (in first-party code or dependencies) must be fixed,
  mitigated, or explicitly accepted with documented, time-boxed justification before a
  release is tagged.
- CI must run automated dependency and vulnerability scanning (e.g. `cargo audit` /
  `cargo deny`) on every pull request and on a recurring schedule, and these checks must
  pass with no unresolved findings before publishing.
- Releases are blocked until any open, confirmed vulnerability report tracked internally
  or via GitHub Security Advisories is resolved.
- If a vulnerability is discovered after a release has been published, a patch release
  addressing it must be issued as soon as possible, following the coordinated disclosure
  timeline above.
- Release maintainers are responsible for verifying — and recording in the release
  checklist — that no open vulnerability reports or failing security scans exist before
  cutting a new release.
