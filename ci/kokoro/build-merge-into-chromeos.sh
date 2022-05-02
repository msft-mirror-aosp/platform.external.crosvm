#!/bin/bash
# Copyright 2021 The Chromium OS Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.
set -e

# Python script to check for at least version 3.9
VERSION_CHECK="
import sys
sys.exit(sys.version_info.major != 3 or sys.version_info.minor < 9)
"

main() {
    cd "${KOKORO_ARTIFACTS_DIR}/git/crosvm"

    # Ensure we have at least python 3.9. Kokoro does not and requires us to use pyenv to install
    # The required version.
    if ! python3 -c "$VERSION_CHECK"; then
        pyenv install --verbose --skip-existing 3.9.5
        pyenv global 3.9.5
    fi

    # Extra packages required by merge_bot
    if ! pip show argh; then
        pip install argh
    fi

    ./tools/chromeos/merge_bot -v update-merges --is-bot
    ./tools/chromeos/merge_bot -v update-dry-runs --is-bot
}

main
