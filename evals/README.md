# Gaggle Evals

Agent evaluation scenarios for Gaggle. This directory is a skeleton mirroring
Goose's evals layout; scenario definitions and runners will be added over time.

## Purpose

- Provide reproducible agent evaluation scenarios for Gaggle (smoke checks,
  task benchmarks, regression suites).
- Keep eval runs isolated from developer data.

## How to run (placeholder)

- Smoke (desktop server): `evals/smoke_eval.sh <path-to-gaggle-desktop-binary>`
- Full scenarios: TBD.

## Safety

- Always run evals against a **temporary database**. Never point evals at a
  real user database (`gaggle.db`). The smoke script uses a throwaway DB in a
  temp directory and cleans it up on exit.
- Use temp directories (`mktemp`) for all artifacts; do not write into the
  repository or user data directories.