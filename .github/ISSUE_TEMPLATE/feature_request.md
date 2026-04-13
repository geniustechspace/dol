---
name: Feature Request
about: Suggest a new feature or enhancement for DOL
title: "[Feature] "
labels: enhancement
assignees: ""
---

## Summary

A clear and concise description of the feature you'd like.

## Motivation

Why is this feature needed? What problem does it solve?

## Proposed Solution

Describe your proposed API or approach.

```rust
// Example usage:
let sql = USERS.get()
    .with_cte("active_users", active_query)
    .all_columns()
    .to_sql(Some(&Dialect::postgres()));
```

## Alternatives Considered

Any alternative approaches you've considered.

## Additional Context

Any other relevant information (links to SQL specs, related issues, etc.).
