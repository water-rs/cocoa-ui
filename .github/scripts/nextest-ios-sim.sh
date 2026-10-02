#!/usr/bin/env bash
# iOS Simulator target runner for the crate's tests.
#
# Wired up through `target.aarch64-apple-ios-sim.runner` in
# `.cargo/config.toml`, which `cargo test` and `cargo nextest run` honour:
# each built test binary is handed here and executed *inside* a booted
# iPhone simulator via `simctl spawn`, so `UIKit`-touching tests run
# natively on the platform — compiling for the simulator is never the
# test. Under nextest every case is its own spawned process, keeping the
# per-case isolation the macOS lane has.
#
# `spawn` runs the Mach-O directly: the suite creates UIKit objects
# (`UIView`, `UILabel`, …) without reaching `UIApplication`, so no `.app`
# bundle is needed. The spawned process's exit status is simctl's, so a
# failing test fails the step.
#
# Usage:  nextest-ios-sim.sh <test-binary> [test args…]
# Env:    COCOA_UI_IOS_SIM_UDID  pin a device; otherwise the first
#         available iPhone in `simctl list` is used (CI always pins a UDID
#         for a device it created itself).
set -euo pipefail

binary="${1:?usage: nextest-ios-sim.sh <test-binary> [test args…]}"
shift

device="${COCOA_UI_IOS_SIM_UDID:-}"
if [[ -z "$device" ]]; then
    device="$(xcrun simctl list devices available | awk -F'[()]' '/iPhone/{print $2; exit}')"
fi
if [[ -z "$device" ]]; then
    echo "nextest-ios-sim: no available iPhone simulator; create one first:" >&2
    echo "  xcrun simctl create cocoa-ui-tests com.apple.CoreSimulator.SimDeviceType.iPhone-16" >&2
    exit 1
fi

# Boot if needed and wait for it to be ready; a no-op when already booted.
# nextest parses this runner's stdout for its list/run protocol, so
# bootstatus chatter must stay off it.
xcrun simctl bootstatus "$device" -b >&2

exec xcrun simctl spawn "$device" "$binary" "$@"
