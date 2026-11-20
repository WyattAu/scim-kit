# Pattern adoption

Declared per the pattern canon adoption format; every claim is verified against code in this repo.

| Pattern | Status | Evidence |
|---|---|---|
| T1 | adopted | src/filter.rs:84 (filter text parsed into a typed AST) + src/pagination.rs:42 (query parsed into typed params) |
| P72 | adopted | src/error.rs:12 (single error enum) + src/error.rs:32 (one status mapping) |
