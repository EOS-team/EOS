#!/bin/sh
# SPDX-License-Identifier: GPL-2.0
#
# rust-version rust-command
#
# Print the compiler version of `rust-command' in a 5 or 6-digit form
# such as `14502' for rustc-1.45.2 etc.
#
# Returns 0 if not found (so that Kconfig does not complain)
compiler="$*"

if [ ${#compiler} -eq 0 ]; then
	echo "Error: No compiler specified." >&2
	printf "Usage:\n\t$0 <rust-command>\n" >&2
	exit 1
fi

if ! command -v $compiler >/dev/null 2>&1; then
	echo 0
	exit 0
fi

VERSION=$($compiler --version | cut -f2 -d' ')

# Cut suffix if any (e.g. `-dev`)
VERSION=$(echo $VERSION | cut -f1 -d'-')

MAJOR=$(echo $VERSION | cut -f1 -d'.')
MINOR=$(echo $VERSION | cut -f2 -d'.')
PATCHLEVEL=$(echo $VERSION | cut -f3 -d'.')
printf "%d%02d%02d\\n" $MAJOR $MINOR $PATCHLEVEL
