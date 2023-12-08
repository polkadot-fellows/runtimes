#!/usr/bin/env python

import os
import sys
import argparse

parser = argparse.ArgumentParser(description="Process the CHANGELOG.md")
parser.add_argument(
    "changelog",
    metavar="CHANGELOG",
    help="Path to the CHANGELOG.md file",
    default="CHANGELOG.md",
    nargs='?'
)

group = parser.add_mutually_exclusive_group()
group.add_argument(
    "--print-latest-version",
    dest="print_latest_version",
    help="Print the latest version (first in the file) found in the CHANGELOG.md",
    action="store_true"
)
group.add_argument(
    "--should-release",
    dest="should_release",
    help="Should a release be made? Prints `1` or `0`.",
    action="store_true"
)
group.add_argument(
    "--print-changelog-until-previous-version",
    dest="changelog_until_previous",
    help="Print the entire changelog until the previous version.",
    action="store_true"
)

args = parser.parse_args()

with open(args.changelog, "r") as changelog:
    lines = changelog.readlines()

    changelog_until_previous_version = ""
    found_last_version = False

    # Find the latest version
    for line in lines:
        if not line.startswith("## ["):
            changelog_until_previous_version += line
            continue
        elif not found_last_version:
            changelog_until_previous_version += line
            found_last_version = True
            version = line.strip().removeprefix("## [").split("]")[0]
        else:
            break


    if args.changelog_until_previous:
        print(changelog_until_previous_version, end = "")
        sys.exit(0)
    elif args.print_latest_version:
        print(version, end = "")
        sys.exit(0)
    elif args.should_release:
        if version.lower() == "unreleased":
            print("0", end = "")
            sys.exit(-1)
        elif version.count(".") != 2:
            print("0", end = "")
            sys.exit(-1)

        stream = os.popen("git tag -l v" + version)
        output = stream.read()

        # Empty output means that the tag doesn't exist and that we should release.
        if output.strip() == "":
            print("1", end = "")
        else:
            print("0", end = "")

        sys.exit(0)
    else:
        parser.print_help()
