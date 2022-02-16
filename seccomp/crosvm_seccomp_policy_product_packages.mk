# Autogenerated via gen_android.sh
#
# Copyright (C) 2020 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

PRODUCT_PACKAGES += \
    9p_device.policy \
    balloon_device.policy \
    battery.policy \
    block_device.policy \
    cras_audio_device.policy \
    fs_device.policy \
    gpu_device.policy \
    input_device.policy \
    net_device.policy \
    null_audio_device.policy \
    pmem_device.policy \
    rng_device.policy \
    serial.policy \
    tpm_device.policy \
    vfio_device.policy \
    vhost_net_device.policy \
    vhost_vsock_device.policy \
    video_device.policy \
    vios_audio_device.policy \
    wl_device.policy \
    xhci.policy \

# TODO: Remove this when crosvm is added to generic system image
PRODUCT_ARTIFACT_PATH_REQUIREMENT_ALLOWED_LIST += \
    system/etc/seccomp_policy/crosvm/9p_device.policy \
    system/etc/seccomp_policy/crosvm/balloon_device.policy \
    system/etc/seccomp_policy/crosvm/battery.policy \
    system/etc/seccomp_policy/crosvm/block_device.policy \
    system/etc/seccomp_policy/crosvm/cras_audio_device.policy \
    system/etc/seccomp_policy/crosvm/fs_device.policy \
    system/etc/seccomp_policy/crosvm/gpu_device.policy \
    system/etc/seccomp_policy/crosvm/input_device.policy \
    system/etc/seccomp_policy/crosvm/net_device.policy \
    system/etc/seccomp_policy/crosvm/null_audio_device.policy \
    system/etc/seccomp_policy/crosvm/pmem_device.policy \
    system/etc/seccomp_policy/crosvm/rng_device.policy \
    system/etc/seccomp_policy/crosvm/serial.policy \
    system/etc/seccomp_policy/crosvm/tpm_device.policy \
    system/etc/seccomp_policy/crosvm/vfio_device.policy \
    system/etc/seccomp_policy/crosvm/vhost_net_device.policy \
    system/etc/seccomp_policy/crosvm/vhost_vsock_device.policy \
    system/etc/seccomp_policy/crosvm/video_device.policy \
    system/etc/seccomp_policy/crosvm/vios_audio_device.policy \
    system/etc/seccomp_policy/crosvm/wl_device.policy \
    system/etc/seccomp_policy/crosvm/xhci.policy \
