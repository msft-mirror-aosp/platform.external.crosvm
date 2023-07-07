#!/usr/bin/env bash
# Copyright 2022 The ChromiumOS Authors
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

# Regenerate kernel_loader bindgen bindings.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

source tools/impl/bindgen-common.sh

KERNEL_LOADER_EXTRA="// Added by kernel_loader/bindgen.sh
use zerocopy::AsBytes;
use zerocopy::FromBytes;"

bindgen_generate \
    --raw-line "${KERNEL_LOADER_EXTRA}" \
    --allowlist-type='Elf32_Ehdr' \
    --allowlist-type='Elf32_Phdr' \
    --allowlist-type='Elf64_Ehdr' \
    --allowlist-type='Elf64_Phdr' \
    --allowlist-var='.+' \
    --with-derive-custom "elf32_hdr=FromBytes,AsBytes" \
    --with-derive-custom "elf64_hdr=FromBytes,AsBytes" \
    --with-derive-custom "elf32_phdr=FromBytes,AsBytes" \
    --with-derive-custom "elf64_phdr=FromBytes,AsBytes" \
    "${BINDGEN_LINUX_X86_HEADERS}/include/linux/elf.h" \
    -- \
    -isystem "${BINDGEN_LINUX_X86_HEADERS}/include" \
    | replace_linux_int_types \
    > kernel_loader/src/elf.rs
