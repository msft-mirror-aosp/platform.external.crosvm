#!/bin/bash
# Copyright 2021 The ChromiumOS Authors
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

source "$(dirname $0)/common.sh"

./tools/dev_container --self-test

./tools/dev_container --hermetic bash -c "\
    ./tools/run_tests --target=host -v \
    && ./tools/health-check \
    && cargo build --verbose --no-default-features \
    && mdbook build ./docs/book \
    && ./tools/cargo-doc"
