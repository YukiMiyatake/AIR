# Contributing

## Branch model

- Default branch: \main- There is **no** \develop\ branch. Do not create or use \develop\.

## Workflow

1. Branch from \main\.
2. Open a pull request into \main\.
3. Wait for the **build-and-test** status check and PR automation.
4. Do **not** push or merge directly to \main\.

Direct pushes to \main\ are blocked by branch protection or repository rulesets.

## Merging

- Squash merge is preferred.
- Auto-merge is enabled after CI passes (see \.github/workflows/pr-automation.yml\).
