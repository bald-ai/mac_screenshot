# AGENTS.md

This file defines the shared rules for AI coding work. Optimize for clarity and fast change, and build for your future AI self.

## Rules (use as defaults)
1) Future-AI clarity: make intent obvious, keep logic easy to find, and add short comments only when the behavior is not self-evident.
2) Feature-first organization: keep code with the feature unless it is truly shared.
3) Consistent naming: use stable, descriptive names; avoid old/new/temp/v2/fixed; keep naming patterns uniform within a feature.
4) Language glossary is the shared source of truth: consult `language.md` for UI/screens/flows. Update it only when the user asks or after a confirmed misunderstanding.
5) Separation by layer: pages wire, components render UI, hooks orchestrate state, lib holds pure logic. Keep core rules testable without React.
6) Name non-obvious or repeated numbers in `constants.ts`; trivial UI math can stay inline.
7) Explicit input validation and clear errors at boundaries (APIs, external data, user input).
8) No `any` in app code; avoid `@ts-ignore` unless documented. Generated code is the exception.
9) Stable UI selectors for key controls (`data-testid`/IDs).
10) No new dependencies or tooling changes without approval.
11) Update docs when behavior changes (short note in existing docs).
12) Gate before handoff: eslint (no lint errors), `npx tsc --noEmit`, plus any existing tests.
13) Coverage bar: Thresholds at 70% for lines/branches/functions/statements.
14) File size guideline: aim to keep files under ~700 LOC; split/refactor when it improves clarity or testability.
15) If documentation.md exists, reference it. It will contain important info when you make plans or change code.
