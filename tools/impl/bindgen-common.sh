# Copyright 2022 The ChromiumOS Authors
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

# Helper functions for bindgen scripts sourced by tools/bindgen-all-the-things.

export BINDGEN_LINUX="${PWD}/../../third_party/kernel/v6.6"

export BINDGEN_PLATFORM2="${PWD}/../../platform2"

export BINDGEN_OPTS=(
    '--disable-header-comment'
    '--no-layout-tests'
    '--no-doc-comments'
    '--with-derive-default'
)

export BINDGEN_HEADER="/* automatically generated by tools/bindgen-all-the-things */

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
"

# Delete definitions of types like __u32 and replace their uses with the equivalent native Rust
# type, like u32. This ensures that the types are correctly sized on all platforms, unlike the
# definitions from the system headers, which resolve to C types like short/int/long that may vary
# across architectures.
replace_linux_int_types() {
    sed -E -e '/^pub type __(u|s)(8|16|32|64) =/d' -e 's/__u(8|16|32|64)/u\1/g' -e 's/__s(8|16|32|64)/i\1/g'
    cat
}

# Delete definitions of types like __le32 and __be32 and replace their uses with the equivalent
# data_model types.
replace_linux_endian_types() {
    sed -E -e '/^pub type __(l|b)e(16|32|64) =/d' -e 's/__le(16|32|64)/Le\1/g' -e 's/__be(16|32|64)/Be\1/g'
}

# Wrapper for bindgen used by the various bindgen.sh scripts.
# Pass extra bindgen options and the .h filename as parameters.
# Output is produced on stdout and should be redirected to a file.
bindgen_generate() {
    echo "${BINDGEN_HEADER}"
    bindgen "${BINDGEN_OPTS[@]}" "$@"
}

bindgen_cleanup() {
    rm -rf "${BINDGEN_LINUX_X86_HEADERS}" "${BINDGEN_LINUX_ARM64_HEADERS}" "${BINDGEN_LINUX_RISCV_HEADERS}"
}

# Install Linux kernel headers for each architecture into temporary locations. These are used for KVM bindings.

if [[ -z "${BINDGEN_LINUX_X86_HEADERS+x}" || ! -d "${BINDGEN_LINUX_X86_HEADERS}" ||
    -z "${BINDGEN_LINUX_ARM64_HEADERS+x}" || ! -d "${BINDGEN_LINUX_ARM64_HEADERS}" ||
    -z "${BINDGEN_LINUX_RISCV_HEADERS+x}" || ! -d "${BINDGEN_LINUX_RISCV_HEADERS}" ]]; then
    export BINDGEN_LINUX_X86_HEADERS='/tmp/bindgen_linux_x86_headers'
    export BINDGEN_LINUX_ARM64_HEADERS='/tmp/bindgen_linux_arm64_headers'
    export BINDGEN_LINUX_RISCV_HEADERS='/tmp/bindgen_linux_riscv_headers'

    trap bindgen_cleanup EXIT

    echo -n "Installing Linux headers for x86, arm64, and riscv..."
    (
        cd "${BINDGEN_LINUX}"
        nproc=$(nproc)
        make -s headers_install ARCH=x86 INSTALL_HDR_PATH="${BINDGEN_LINUX_X86_HEADERS}" -j "${nproc}"
        make -s headers_install ARCH=arm64 INSTALL_HDR_PATH="${BINDGEN_LINUX_ARM64_HEADERS}" -j "${nproc}"
        make -s headers_install ARCH=riscv INSTALL_HDR_PATH="${BINDGEN_LINUX_RISCV_HEADERS}" -j "${nproc}"
        make -s mrproper
    )
    echo " done."
fi
