#!/usr/bin/env bash
# ***
#
# Helios Mesh
#
# SPDX-License-Identifier: Apache-2.0
#
# ***

# Install Native Dependencies For The Pinned elf2uf2-rs Serial Feature.
set -euo pipefail

privilege=()
if [[ $(id -u) -ne 0 ]]; then
  privilege=(sudo)
fi

"${privilege[@]}" apt-get update
"${privilege[@]}" apt-get install --yes --no-install-recommends libudev-dev pkg-config
