---
created: 2026-10-25T11:03:02.538Z
title: Re-check Dependabot after first merge to main
area: tooling
severity: minor
files:
  - .github/dependabot.yml
  - .planning/phases/01-development-foundation/01-UAT.md
---

## Problem

Phase 1 configured Dependabot version updates (github-actions and the devcontainer image, grouped,
weekly, `target-branch: develop`), but Dependabot reads `.github/dependabot.yml` only from the
default branch (`main`), which does not have it yet. So the behaviour could not be observed when
phase 1 was verified. Recorded as a deferred follow-up in 01-UAT.md test 2 and 01-VERIFICATION.md.

## Solution

After the first release merge to main: check the repository's Dependabot update log (Insights >
Dependency graph > Dependabot, or `gh api` for the repository's dependabot data) and confirm that
grouped update PRs for github-actions and the devcontainer image are opened against `develop`.
A run that errors or targets `main` is a defect in the config.
