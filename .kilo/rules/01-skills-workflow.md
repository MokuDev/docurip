# Skills Workflow

This project ships 24 engineering skills in `.kilo/skills/`. A skill is a mandatory workflow — not a suggestion. When a task matches a skill, follow it exactly.

## The One Rule

**Check for a matching skill before doing anything else.** Not after gathering context, not after writing the first line — before. A 1% chance that a skill applies is enough to check.

## Intent → Skill

Map what the user is asking to the right skill:

| User says / wants | Skill to use |
|---|---|
| New feature, new page, new endpoint | `spec-driven-development` |
| Break down into tasks / what order to build | `planning-and-task-breakdown` |
| Implement, build, code it | `incremental-implementation` + `test-driven-development` |
| Write tests / fix failing tests | `test-driven-development` |
| Something broken, unexpected behavior, 500 error | `debugging-and-error-recovery` |
| Review code, find issues, pre-merge check | `code-review-and-quality` |
| Simplify, refactor, reduce complexity | `code-simplification` |
| Design an API or interface | `api-and-interface-design` |
| Frontend, UI, component | `frontend-ui-engineering` |
| Security audit, vulnerability check | `security-and-hardening` |
| Performance, slow, bundle size | `performance-optimization` |
| Logging, metrics, tracing | `observability-and-instrumentation` |
| Git, commit, PR, branch | `git-workflow-and-versioning` |
| CI/CD, pipeline, deployment | `ci-cd-and-automation` |
| Deprecate, migrate, remove | `deprecation-and-migration` |
| Docs, ADR, decision record | `documentation-and-adrs` |
| Ready to ship, launch checklist | `shipping-and-launch` |
| Risky change, not sure how to proceed | `doubt-driven-development` |
| Explore an idea, brainstorm, validate concept | `idea-refine` |
| Need to understand a complex codebase | `context-engineering` |
| Follow existing code patterns | `source-driven-development` |
| Explore what the user actually wants | `interview-me` |

## Development Lifecycle — Recommended Order

For greenfield features or non-trivial changes, follow this sequence:

```
DEFINE   →  spec-driven-development          (/spec)
PLAN     →  planning-and-task-breakdown      (/plan)
BUILD    →  incremental-implementation       (/build)
         +  test-driven-development          (RED → GREEN → refactor)
VERIFY   →  debugging-and-error-recovery     (if something breaks)
         +  browser-testing-with-devtools    (UI/browser issues)
REVIEW   →  code-review-and-quality          (/review)
         +  code-simplification              (/code-simplify)
         +  security-and-hardening           (auth, inputs, secrets)
         +  performance-optimization         (/webperf for web)
SHIP     →  git-workflow-and-versioning      (commit, PR)
         +  shipping-and-launch              (/ship — parallel fan-out)
```

You do not need every phase for every task. A small bug fix starts at VERIFY. A one-file refactor starts at REVIEW. Use judgment — but always go through the phase, not around it.

## Slash Commands

The commands in `.kilo/command/` are entry points into this lifecycle:

| Command | Phase | Skills invoked |
|---|---|---|
| `/spec` | DEFINE | `spec-driven-development` |
| `/plan` | PLAN | `planning-and-task-breakdown` |
| `/build` | BUILD | `incremental-implementation` + `test-driven-development` |
| `/build auto` | BUILD | Same, runs the full plan in one approved pass |
| `/test` | BUILD/VERIFY | `test-driven-development` |
| `/review` | REVIEW | `code-review-and-quality` |
| `/code-simplify` | REVIEW | `code-simplification` |
| `/webperf` | REVIEW | `web-performance-auditor` agent |
| `/ship` | SHIP | `shipping-and-launch` (spawns `code-reviewer`, `security-auditor`, `test-engineer` in parallel) |

## Agents

Select a specialist agent from `.kilo/agents/` when the task calls for a focused review without implementation:

| Agent | When to use |
|---|---|
| `code-reviewer` | Pre-merge review, architectural feedback |
| `security-auditor` | Auth code, input handling, secrets, OWASP check |
| `test-engineer` | Test strategy, coverage gaps, test quality |
| `web-performance-auditor` | Core Web Vitals, loading, rendering, bundle analysis |

These agents are also invoked automatically by `/ship` (parallel fan-out).

## Skills Are Mandatory, Not Optional

These rationalizations are always wrong — ignore them:

- "This is too small for a skill" → skills apply at any size
- "I'll just quickly implement it" → implementation without a skill is the wrong path
- "Let me gather context first, then check skills" → check skills first, they tell you how to gather context
- "The skill feels like overkill here" → follow it anyway; shortcuts compound into bugs
- "I know this pattern, I don't need the skill" → the skill enforces discipline you'll thank later

## How to Invoke a Skill

Name it explicitly in your reasoning or tell the user:

> "I'll use the `test-driven-development` skill for this."

Then follow the skill's process exactly — every step, in order. Do not cherry-pick steps. If a step seems unnecessary for this specific case, follow it anyway unless the skill itself says it's optional.
