# Process for removing runtime migrations

We want to keep the `migrations` lists in the runtimes nice and tidy, without keeping around a lot of migration code once we know it's no longer needed - meaning once it's been enacted on-chain.

The following is a quick guide/process for removing applied migrations while making sure you don't remove migrations not yet applied.

## Prerequisites

For some chain runtime `spec_version: a_bcd_efg,` (e.g. `spec_version: 1_000_001`):
- has been officially released on https://github.com/polkadot-fellows/runtimes/releases/ as part of `Runtimes X.Y.Z` release/tag.
- the **on-chain** runtime version has been upgraded to spec version `a_bcd_efg` (using wasm blob released above).

## Steps

1. Sync tags: `git pull upstream main --tags`,
2. Check-out **the released** code: `git checkout vX.Y.Z`,
   - This is required to make sure you are not accidentally removing yet unreleased migrations (PRs merged between `X.Y.Z` release and when you are doing this).
3. Create patch with your changes: `git diff --patch > remove-migrations.patch`,
4. Now `git checkout main` and apply your patch `git am -3 remove-migrations.patch` or `git apply -3 remove-migrations.patch`,
   - thus ensuring you are not removing any migrations merged to main after the release.
5. `git checkout -b <PR-branch-bla>`, `git push --set-upstream origin <PR-branch-bla>`, then open PR.

## Automation

Currently, all of the above is done manually, but (at least parts of) it could be automated. Prerequisites can definitely be automated, so could the branches/patches dance. Code changes would easily be done manually and that's it.