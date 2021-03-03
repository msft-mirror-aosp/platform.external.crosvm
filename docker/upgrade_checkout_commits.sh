#!/bin/bash
# Copyright 2019 The Chromium OS Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

cd "${0%/*}"

remotes=(
    "https://github.com/mesonbuild/meson"
    "https://github.com/anholt/libepoxy.git"
    "https://chromium.googlesource.com/chromiumos/third_party/tpm2"
    "https://chromium.googlesource.com/chromiumos/platform2"
    "https://chromium.googlesource.com/chromiumos/third_party/adhd"
    "https://gitlab.freedesktop.org/mesa/drm.git/"
    "https://android.googlesource.com/platform/external/minijail"
    "https://gitlab.freedesktop.org/virgl/virglrenderer.git"
    "https://chromium.googlesource.com/chromiumos/platform/minigbm"
)

keys=(
    "MESON_COMMIT"
    "LIBEPOXY_COMMIT"
    "TPM2_COMMIT"
    "PLATFORM2_COMMIT"
    "ADHD_COMMIT"
    "DRM_COMMIT"
    "MINIJAIL_COMMIT"
    "VIRGL_COMMIT"
    "MINIGBM_COMMIT"
)

for ((i = 0; i < ${#remotes[*]}; ++i)); do
    remote="${remotes[$i]}"
    key="${keys[$i]}"
    branch="${branches[$i]}"
    remote_chunk=$(git ls-remote --exit-code "${remote}" "refs/heads/main")
    if [ -z "${remote_chunk}" ]; then
        remote_chunk=$(git ls-remote --exit-code "${remote}" \
            "refs/heads/master")
    fi
    commit=$(echo "${remote_chunk}" | cut -f 1 -)
    echo $key=$commit
done
