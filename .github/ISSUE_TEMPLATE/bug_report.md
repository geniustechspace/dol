---
name: Bug Report
about: Report a bug in DOL
title: "[Bug] "
labels: bug
assignees: ""
---

## Description

A clear and concise description of the bug.

## Steps to Reproduce

1. Define a model with ...
2. Call `.get().where_eq("field").to_sql(Some(&Dialect::postgres()))`
3. Observe incorrect output

## Expected Behavior

What you expected to happen.

## Actual Behavior

What actually happened. Include the generated SQL if applicable.

## Environment

- **DOL version:** 0.1.x
- **Rust version:** (output of `rustc --version`)
- **OS:** (e.g., Ubuntu 24.04, macOS 15)

## Additional Context

Any other relevant information (e.g., dialect, feature flags).
