# Runtimes

This repository houses the code required to build the runtimes for Polkadot, Kusama, and their System-Parachains. Its maintenance is overseen by the Fellowship, as decreed by the Polkadot and Kusama Governance. The primary objective is to provide excellent code, which can subsequently be enacted on-chain through a decentralized referendum.

## Structure

Runtimes can be found in the `relay` and `system-parachains` top-level folders, each leaf folder of which contains one runtime crate:

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
    ├── coretime
    │   ├── coretime-kusama
    │   └── coretime-polkadot
    ├── encointer
    ├── gluttons
    │   └── glutton-kusama
    └── people
        ├── people-kusama
        └── people-polkadot
```

## Approval rights

The approval rights are configured in [`review-bot.yml`](.github/review-bot.yml). The rights are configured as:

- All files in `.github` require two approvals from Fellowship members of rank 4 or higher.
- `CHANGELOG.md`, `relay/*` or `system-parachains/*` require four approvals from Fellowship members of rank 3 or higher.
- All other files require the approval from one Fellowship member of rank 2 or higher.

The review-bot uses the on-chain identity to map from a GitHub account to a Fellowship member. This requires that each Fellowship member add their GitHub handle to their on-chain identity. Check [here](docs/on-chain-identity.md) for instructions.

- [Official List of Fellows](https://polkadot-fellows.github.io/dashboard/#/members)
- [List of Fellows with their GitHub handles](https://fellowship.tasty.limo/)

## Working on Pull Requests

To merge a pull request, we use [Auto Merge Bot](https://github.com/paritytech/auto-merge-bot).

To use it, write a comment in a PR that says:

> `/merge`

This will enable [`auto-merge`](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/automatically-merging-a-pull-request) in the Pull Request (or merge it if it is ready to merge).

The automation can be triggered by the author of the PR or any fellow whose GitHub handle is part of their identity.

## Release process

Releases are automatically pushed on commits merged to master that fulfill the following requirements:

- The [`CHANGELOG.md`](CHANGELOG.md) file was modified.
- The latest version (the version at the top of the file) in [`CHANGELOG.md`](CHANGELOG.md) has no tag in the repository.

The release process is building all runtimes and then puts them into a release in this github repository.

The format of [`CHANGELOG.md`](CHANGELOG.md) is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

### Point releases

In order to release a patch or fix outside the normal release process, some additional steps are needed.

To submit a fix to release `x.y.z` and make a point release:

- Make your changes based on x.y.z
- Create a branch `x.y.b` from the most recent release, where `b = z + 1` (contact a maintainer)
- Make a PR against the branch `x.y.b`
- Trigger the release process manually (contact a maintainer)
- Check for other planned releases which originally targeted the same semver version and post on the issue letting them know that they should bump
- Once the release is out, amend the GitHub release and delete all unchanged runtime blobs. Highlight if this release only affects some runtimes (contact a maintainer)
- Backport your changes to the `CHANGELOG.md` to the main branch
## Release guidelines

Here is an overview of the recommended steps.

|Steps |Description |
|------|------------|
|0 |Open an [issue](https://github.com/polkadot-fellows/runtimes/issues) for the release in the runtimes repo with the **version number**. |
|1 |Update **[polkadot-sdk](https://github.com/paritytech/polkadot-sdk?tab=readme-ov-file#-releases)**, if applicable. |
|2 |Identify and monitor **potential blockers** (old dependencies, pending or failed upgrades). |
|3 |Identify and include **PRs** with required tests, highlighting the integration tests that have changed. |
|4 |Identify and communicate all details about **potential breaking changes** (transaction/event/error encoding, polkadot-sdk migrations, XCM and storage format, etc.) or **disruptions**. Make sure to **ping @SBalaguer and @anaelleltd** in your commentary. |
|5 |Run **[benchmarking](https://github.com/polkadot-fellows/runtimes/blob/main/docs/weight-generation.md)** for changed pallets. |
|6 |Trigger the release for **final reviews**, making sure to highlight information about all breaking changes or disruptions in the **CHANGELOG entry**. |
|7 |Create the **[whitelisting proposal (Fellowship)](https://github.com/joepetrowski/opengov-cli)** with contextual information.|
|8 |Create the **[whitelisted caller referendum (OpenGov)](https://github.com/joepetrowski/opengov-cli)** with contextual information and **instructions for following up** on breaking changes or disruptions. |
|9 |Close the issue for the release once the referendum is **approved and executed**. |
|10 |Open an issue for **the next release** in the runtimes repo, if applicable.|


## Communication channels

The Fellowship is using Matrix for communication. Right now there exists two channels:

- [Polkadot Technical Fellowship Channel](https://matrix.to/#/#fellowship-members:parity.io): The channel for all Fellowship members to discuss. To get voice rights, you need to be part of the Fellowship. However, the channel is readable by anyone.
- [Polkadot Technical Fellowship - Open Channel](https://matrix.to/#/#fellowship-open-channel:parity.io): Open channel for anyone. Should be used to reach out to the Fellowship e.g. to request review or help on a topic.
