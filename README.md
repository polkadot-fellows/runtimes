# Runtimes

This repository houses the code required to build the runtimes for Polkadot, Kusama, and their System-Parachains. Its maintenance is overseen by the Fellowship, as decreed by the Polkadot and Kusama Governance. The primary objective is to provide excellent code, which can subsequently be enacted on-chain through a decentralized referendum.

## Structure

Each leaf folder contains one runtime crate:

<!-- Run "tree -I 'target' -d -L 3" and then delete some folders from Polkadot and Kusama. -->

```pre
├── relay
│   ├── kusama
│   └── polkadot
└── system-parachains
    ├── asset-hubs
    │   ├── asset-hub-kusama
    │   └── asset-hub-polkadot
    ├── bridge-hubs
    │   ├── bridge-hub-kusama
    │   └── bridge-hub-polkadot
    ├── collectives
    │   └── collectives-polkadot
    └── gluttons
        └── glutton-kusama
```

## Approval rights

The approval rights are configured in [`review-bot.yml`](.github/review-bot.yml). The rights are configured as:

- All files in `.github` require two approvals from Fellowship members of rank 4 or higher.
- `CHANGELOG.md`, `relay/*` or `system-parachains/*` require four approvals from Fellowship members of rank 3 or higher.
- All other files require the approval from one Fellowship member of rank 2 or higher.

The review-bot uses the on-chain identity to map from a GitHub account to a Fellowship member. This requires that each Fellowship member add their GitHub handle to their on-chain identity. Check [here](docs/on-chain-identity.md) for instructions.

# Working on Pull Requests

To merge a pull request, we use [Auto Merge Bot](https://github.com/paritytech/auto-merge-bot).

To use it, write a comment in a PR that says:

> `/merge`

This will enable [`auto-merge`](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/automatically-merging-a-pull-request) in the Pull Request (or merge it if it is ready to merge).

The automation can be triggered by the author of the PR or any fellow whose GitHub handle is part of their identity.

# Release process

Releases are automatically pushed on commits merged to master that fulfill the following requirements:

- The [`CHANGELOG.md`](CHANGELOG.md) file was modified.
- The latest version (the version at the top of the file) in [`CHANGELOG.md`](CHANGELOG.md) has no tag in the repository.

The release process is building all runtimes and then puts them into a release in this github repository.

The format of [`CHANGELOG.md`](CHANGELOG.md) is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
