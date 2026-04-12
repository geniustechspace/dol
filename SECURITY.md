# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in DOL, please report it responsibly.

**Do NOT open a public issue.**

Instead, please email the maintainers at: **security@geniustechspace.com**

Or use [GitHub's private vulnerability reporting](https://github.com/geniustechspace/dol/security/advisories/new).

### What to include

- A description of the vulnerability.
- Steps to reproduce the issue.
- The potential impact and severity.
- Any suggested fix, if you have one.

### Response Timeline

- **Acknowledgement:** Within 48 hours of receipt.
- **Assessment:** Within 7 days we will confirm whether the report is valid.
- **Fix & Disclosure:** We aim to release a fix within 30 days for confirmed
  vulnerabilities. We will coordinate disclosure with you.

## Security Measures

DOL employs the following security practices:

- **`cargo-deny`** — Checks for known vulnerable dependencies on every PR.
- **`cargo-audit`** — Weekly dependency vulnerability scanning.
- **Trivy** — Repository-wide vulnerability scanning with SARIF reporting.
- **`#![deny(unsafe_code)]`** — Unsafe Rust is forbidden in all crates.
- **No network I/O** — DOL is a pure query/schema language with no runtime
  execution, minimizing attack surface.

## Scope

This policy covers the `dol` workspace and all its published crates. It does
**not** cover:

- Third-party dependencies (report those to their respective maintainers).
- Applications built on top of DOL.
