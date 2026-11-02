# 0157: GitHub Pages deploys reliably

status: open
agent: -
branch: issue/0157-pages-deploy
depends-on: -
guide: -
touches: .github/workflows/docs.yml

## Goal
Owner: "the GitHub Pages no longer deploy." Findings by the coordinator: the latest docs run
(41544476664) failed in `actions/deploy-pages@v4` with "Failed to get ID Token … socket
hang up" (a transient OIDC error; permissions are correct). A re-run deployed. The site
also looked stale because main had local merges not yet pushed (pushed now).

Make the workflow robust: retry the deploy step on transient failure (e.g. a second
deploy-pages step that runs only if the first failed, after a short wait), keep the
concurrency group, and make the build job's checkout use the pushed commit as today.
Check the workflow with `actionlint` if available (or by careful review), keep it small.

## Acceptance criteria
- [ ] Workflow change merged; a short note in the workflow's header comment on why the retry exists.

## Log

## Result
