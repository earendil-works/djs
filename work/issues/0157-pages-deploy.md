# 0157: GitHub Pages deploys reliably

status: done
agent: pages
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
- [x] Workflow change merged; a short note in the workflow's header comment on why the retry exists.

## Log
- Deploy job: first deploy-pages step has continue-on-error; on failure, sleep 30 s and run a second deploy-pages step (id retry); environment url takes either output. Header comment explains. actionlint clean. Concurrency and build job unchanged. No guide pages affected.

## Result
Deploy retried once after 30 s on failure (.github/workflows/docs.yml). Gap: a deployment that fails partway could in theory leave the first attempt's deployment pending; deploy-pages cancels its deployment on failure, so the retry starts clean.
