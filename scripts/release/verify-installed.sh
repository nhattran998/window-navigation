#!/usr/bin/env bash
# Self-verify an installed Wnav.app end to end on this machine.
#
# Loop: (re)install the app from a DMG or a locally built bundle → quit any
# running copy → launch → wait for the menu-bar status item → drive a real
# target window (TextEdit) with REAL global key events (CGEventPost, not the
# CLI) → assert the resulting AX frame → also exercise the status menu →
# write a JSON report.
#
# Usage: verify-installed.sh [--dmg <file>] [--app <Wnav.app>] [--loop N] [--sleep S]
#   --dmg   install from this DMG (default: newest wnav-*.dmg in ~/Downloads)
#   --app   install this already-built .app bundle instead of a DMG
#   --loop  run the whole cycle N times (default 1)
#   --sleep seconds between loops (default 0)
#
# Requires: Accessibility + Automation permission for the terminal running
# this script (it posts CGEvents and drives System Events).
set -euo pipefail
cd "$(dirname "$0")/../.."

APP_NAME="Wnav"
INSTALL_DIR="/Applications/${APP_NAME}.app"
REPORT_DIR="${WNAV_VERIFY_REPORT_DIR:-$PWD/target/verify}"
HELPER="${TMPDIR:-/tmp}/wnav-verify-key"
CONFIG_PATH="${WNAV_CONFIG:-$HOME/.config/wnav/config.toml}"
CONFIG_BACKUP=""

log() { printf '[verify] %s\n' "$*" >&2; }
die() { log "FAIL: $*"; exit 1; }

# ---------------------------------------------------------------- options ---
dmg=""
app_src=""
loops=1
loop_sleep=0
no_install=0
process_name="Wnav"
while [ $# -gt 0 ]; do
    case "$1" in
        --dmg) dmg="$2"; shift 2 ;;
        --app) app_src="$2"; shift 2 ;;
        --loop) loops="$2"; shift 2 ;;
        --sleep) loop_sleep="$2"; shift 2 ;;
        # Drive an app that is already running (e.g. a dev build that holds
        # an Accessibility grant) without reinstalling anything.
        --no-install) no_install=1; shift ;;
        --process) process_name="$2"; shift 2 ;;
        *) die "unknown argument: $1" ;;
    esac
done
PROC="$APP_NAME"
if [ "$no_install" = 1 ]; then PROC="$process_name"; fi

# -------------------------------------------------------------- key helper ---
# CGEventPost helper: real key events, independent of which process is
# frontmost. Args: keycode [cmd] [alt] [ctrl] [shift]
#
# Two properties matter and were both learned the hard way:
#   1. Post to the SESSION tap (.cgSessionEventTap). Events posted to the HID
#      tap (.cghidEventTap) look fine on an event tap but arrow keys never get
#      delivered to any app on macOS 26, which makes a healthy Wnav look
#      broken on exactly its default shortcuts.
#   2. Post modifiers as separate key events around the main key, the way a
#      physical keyboard does, instead of only setting flags on one event.
build_key_helper() {
    if [ -x "$HELPER" ]; then return; fi
    log "building key helper"
    cat > "$HELPER.swift" <<'SWIFT'
import Foundation
import CoreGraphics

let args = CommandLine.arguments
guard args.count >= 2, let code = UInt16(args[1]) else { exit(2) }

// kVK_* virtual keycodes for the modifiers, in press order.
var mods: [(key: UInt16, flag: CGEventFlags)] = []
if args.contains("ctrl") { mods.append((0x3B, .maskControl)) }
if args.contains("alt") { mods.append((0x3A, .maskAlternate)) }
if args.contains("shift") { mods.append((0x38, .maskShift)) }
if args.contains("cmd") { mods.append((0x37, .maskCommand)) }
let flags = mods.reduce(CGEventFlags()) { $0.union($1.flag) }

let source = CGEventSource(stateID: .privateState)

func post(_ key: UInt16, down: Bool, _ f: CGEventFlags) {
    guard let e = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: down) else { return }
    e.flags = f
    // SESSION tap, not the HID tap: on macOS 26 arrow keys posted to
    // .cghidEventTap are never delivered as key input to any app.
    e.post(tap: .cgSessionEventTap)
}

// Hold modifiers: key down with progressively accumulated flags.
var held = CGEventFlags()
for m in mods {
    held.insert(m.flag)
    post(m.key, down: true, held)
    usleep(15_000)
}
// The key itself, with all modifiers held. Arrow keys are numpad keys: a
// physical arrow carries fn + numericPad, and Carbon matches the modifier
// word exactly, so an arrow WITHOUT these never fires (while letters must
// NOT carry them). Measured 3/3 vs 0/3 each way.
let arrows: Set<UInt16> = [123, 124, 125, 126]
var keyFlags = flags
if arrows.contains(code) { keyFlags.insert(.maskSecondaryFn); keyFlags.insert(.maskNumericPad) }
post(code, down: true, keyFlags)
usleep(30_000)
post(code, down: false, keyFlags)
usleep(15_000)
// Release modifiers in reverse order.
for m in mods.reversed() {
    held.subtract(m.flag)
    post(m.key, down: false, held)
    usleep(15_000)
}
SWIFT
    xcrun swiftc -O -o "$HELPER" "$HELPER.swift"
    rm -f "$HELPER.swift"
}

key() { "$HELPER" "$@"; }

# macOS virtual keycodes (kVK_*).
KEY_LEFT=123; KEY_RIGHT=124; KEY_DOWN=125; KEY_UP=126
KEY_RETURN=36

# Default shortcuts (crates/wnav-core/src/preset.rs): ctrl+alt+<key>.
#
# Every press: secure focus, send the key with no delay in between, then
# record which app was frontmost at the moment the key landed. A global hotkey
# acts on the FRONTMOST app, so FOCUSED_AT_KEY is what makes a result
# attributable: if TextEdit did not hold focus, the check is about the wrong
# window and must not be read as a Wnav failure.
FOCUSED_AT_KEY=""
press() {
    ensure_focus || { FOCUSED_AT_KEY="unfocused"; return 1; }
    key "$@"
    FOCUSED_AT_KEY=$(target_focused)
}
press_left()     { press "$KEY_LEFT" ctrl alt; }
press_right()    { press "$KEY_RIGHT" ctrl alt; }
press_up()       { press "$KEY_UP" ctrl alt; }
press_down()     { press "$KEY_DOWN" ctrl alt; }
press_maximize() { press "$KEY_RETURN" ctrl alt; }

# --------------------------------------------------------------- install ----
quit_app() {
    if pgrep -xq "$APP_NAME"; then
        osascript -e "tell application \"$APP_NAME\" to quit" 2>/dev/null || pkill -x "$APP_NAME" || true
    fi
    for _ in $(seq 1 30); do
        pgrep -xq "$APP_NAME" || return 0
        sleep 0.2
    done
    pkill -9 -x "$APP_NAME" || true
}

install_from_dmg() {
    local dmg_file="$1"
    [ -f "$dmg_file" ] || die "dmg not found: $dmg_file"
    local mount
    mount=$(mktemp -d "${TMPDIR:-/tmp}/wnav-verify-mnt.XXXXXX")
    hdiutil attach -quiet -nobrowse -readonly -mountpoint "$mount" "$dmg_file"
    quit_app
    rm -rf "$INSTALL_DIR"
    ditto "$mount/$APP_NAME.app" "$INSTALL_DIR"
    hdiutil detach -quiet "$mount"
    rmdir "$mount" 2>/dev/null || true
    xattr -c "$INSTALL_DIR"
}

install_from_app() {
    local src="$1"
    [ -d "$src" ] || die "app bundle not found: $src"
    quit_app
    rm -rf "$INSTALL_DIR"
    ditto "$src" "$INSTALL_DIR"
    xattr -c "$INSTALL_DIR"
}

# --------------------------------------------------------- config fixture ---
# The verification must run against the DEFAULT shortcuts, whatever the user
# has remapped. Back the real config up and restore it on exit.
backup_config() {
    [ -n "$CONFIG_BACKUP" ] && return
    if [ -f "$CONFIG_PATH" ]; then
        CONFIG_BACKUP=$(mktemp "${TMPDIR:-/tmp}/wnav-verify-cfg.XXXXXX")
        cp "$CONFIG_PATH" "$CONFIG_BACKUP"
        rm -f "$CONFIG_PATH"
    fi
}
restore_config() {
    if [ -z "$CONFIG_BACKUP" ]; then return 0; fi
    mkdir -p "$(dirname "$CONFIG_PATH")"
    mv "$CONFIG_BACKUP" "$CONFIG_PATH"
    CONFIG_BACKUP=""
}
cleanup() {
    # Always return success: an EXIT trap's status replaces the script's.
    osascript -e 'tell application "TextEdit" to close every window saving no' >/dev/null 2>&1
    restore_config
    return 0
}
trap cleanup EXIT

# ------------------------------------------------------------ observation ---
installed_version() {
    /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$INSTALL_DIR/Contents/Info.plist" 2>/dev/null || echo "?"
}

# Wait until the app process exists AND its status item menu is enumerable.
wait_for_status_item() {
    for _ in $(seq 1 100); do
        pgrep -xq "$PROC" || { sleep 0.2; continue; }
        if osascript -e "tell application \"System Events\" to tell process \"$PROC\" to get name of menu bar item 1 of menu bar 2" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.2
    done
    return 1
}

# TCC's verdict on Accessibility for the running app, read from tccd's log.
#
# WHY THIS MATTERS: the grant is stored as a code requirement. An ad-hoc
# signed app stores `cdhash H"…"`, pinning the grant to that one binary, so a
# new build no longer matches ("Failed to match existing code requirement")
# and every hotkey and menu action silently no-ops. Releases are now signed
# with an identifier-only requirement so a grant survives later builds.
#
# HOW IT IS READ: each request the app makes is logged by tccd, keyed by msgID:
#   AUTHREQ_CTX:    msgID=<pid>.<n>, … service=kTCCServiceAccessibility …
#   AUTHREQ_RESULT: msgID=<pid>.<n>, authValue=0|2 …   (0 denied, 2 allowed)
# The app also requests other services (ListenEvent, Apple Events) that are
# allowed independently, so a verdict counts ONLY for requests whose CTX names
# Accessibility. Neither the pid alone nor the "subject" identifies the
# service: the subject is attributed to the responsible process (the terminal
# that launched the app), not to the app.
# Only lines since this process started are considered.
ax_trusted() {
    local pid started
    # `pgrep` exits 1 on no match; `|| true` keeps set -e from aborting here.
    pid=$(pgrep -x "$PROC" 2>/dev/null | head -1 || true)
    [ -n "$pid" ] || { echo "notrunning"; return; }
    # `ps` pads lstart with trailing spaces and date(1) rejects them.
    started=$(date -j -f '%a %b %d %T %Y' "$(ps -o lstart= -p "$pid" | sed -E 's/^[[:space:]]*//; s/[[:space:]]+$//')" '+%Y-%m-%d %H:%M:%S' 2>/dev/null)
    # An unparseable launch time must never read as a verdict.
    [ -n "$started" ] || { echo "unknown"; return; }

    # Poll briefly until this process has an Accessibility request on record
    # (the app asks a moment after launch).
    local log_window id verdict
    for _ in 1 2 3 4 5 6 7 8 9 10; do
        log_window=$(/usr/bin/log show --start "$started" --style compact --predicate 'process == "tccd"' 2>/dev/null || true)
        if printf '%s\n' "$log_window" | grep -q "AUTHREQ_CTX: msgID=$pid\.[0-9]*, .*service=kTCCServiceAccessibility"; then
            break
        fi
        sleep 1
    done
    for id in $(printf '%s\n' "$log_window" \
        | grep "AUTHREQ_CTX: msgID=$pid\.[0-9]*, .*service=kTCCServiceAccessibility" \
        | grep -o "msgID=[0-9.]*" | cut -d= -f2 || true); do
        case "$(printf '%s\n' "$log_window" | grep "AUTHREQ_RESULT: msgID=$id," | grep -o 'authValue=[0-9]*' | cut -d= -f2)" in
            0) verdict="denied" ;;
            2) verdict="granted" ;;
        esac
    done
    # Last Accessibility request wins (the most recent state of the grant).
    echo "${verdict:-unknown}"
}

# True when every check failed AND the trust probe said denied: the definitive
# "fresh install cannot act" signature.
fresh_install_denied() { [ "$TRUST" = "denied" ] && [ "$FAIL" -gt 0 ] && [ "$PASS" -eq 0 ]; }

# Frame of TextEdit's document window via AX: "x,y,w,h".
target_frame() {
    osascript <<'OSA' 2>/dev/null
tell application "System Events"
    tell process "TextEdit"
        set w to first window whose subrole is "AXStandardWindow"
        set {x, y} to position of w
        set {wd, ht} to size of w
        return (x as text) & "," & (y as text) & "," & (wd as text) & "," & (ht as text)
    end tell
end tell
OSA
}

target_focused() {
    osascript -e 'tell application "System Events" to get name of first process whose frontmost is true' 2>/dev/null
}

# Make TextEdit the frontmost app and confirm it, because a global hotkey and
# the status menu both act on whichever app IS frontmost. Another app grabbing
# focus mid-test would otherwise have Wnav correctly move THAT app's window
# while the assertions watch TextEdit, which then looks "stuck".
# Returns 0 once TextEdit is verifiably frontmost, 1 if it never became so.
ensure_focus() {
    for _ in 1 2 3 4 5 6 7 8 9 10; do
        [ "$(target_focused)" = "TextEdit" ] && return 0
        osascript -e 'tell application "TextEdit" to activate' >/dev/null 2>&1
        sleep 0.3
    done
    return 1
}

within() { awk -v a="$1" -v b="$2" -v t="${3:-2}" 'BEGIN { exit (a - b > t || b - a > t) ? 1 : 0 }'; }

# -------------------------------------------------------------- test step ---
# check <name> <expected x,y,w,h> ; reads actual frame and compares each edge.
PASS=0
FAIL=0
INCONCLUSIVE=0
FAILURES=()
RESULTS=""

check() {
    local name="$1" ex="$2"
    local actual
    actual=$(target_frame)
    if [ -z "$actual" ]; then
        FAIL=$((FAIL + 1)); FAILURES+=("$name: no TextEdit window")
        RESULTS="$RESULTS{\"name\":\"$name\",\"ok\":false,\"expected\":\"$ex\",\"actual\":null},"
        log "  ✗ $name — no window"; return 1
    fi
    IFS=, read -r ax ay aw ah <<<"$actual"
    IFS=, read -r ex_x ex_y ex_w ex_h <<<"$ex"
    local ok=1 part
    for part in "$ax $ex_x" "$ay $ex_y" "$aw $ex_w" "$ah $ex_h"; do
        within "${part% *}" "${part#* }" 2 || ok=0
    done
    if [ "$ok" = 1 ]; then
        PASS=$((PASS + 1)); RESULTS="$RESULTS{\"name\":\"$name\",\"ok\":true,\"expected\":\"$ex\",\"actual\":\"$actual\"},"
        log "  ✓ $name → $actual"
    elif [ "$FOCUSED_AT_KEY" != "TextEdit" ]; then
        # The key went to whichever app held focus, so this says nothing
        # about Wnav. Not a failure and not a pass.
        INCONCLUSIVE=$((INCONCLUSIVE + 1))
        FAILURES+=("$name: inconclusive, '$FOCUSED_AT_KEY' was frontmost when the key landed")
        RESULTS="$RESULTS{\"name\":\"$name\",\"ok\":null,\"expected\":\"$ex\",\"actual\":\"$actual\",\"frontmost\":\"$FOCUSED_AT_KEY\"},"
        log "  ? $name — inconclusive: '$FOCUSED_AT_KEY' held focus when the key landed, not TextEdit"
    else
        FAIL=$((FAIL + 1)); FAILURES+=("$name: expected $ex, got $actual")
        RESULTS="$RESULTS{\"name\":\"$name\",\"ok\":false,\"expected\":\"$ex\",\"actual\":\"$actual\"},"
        log "  ✗ $name — expected $ex, got $actual"
    fi
    [ "$ok" = 1 ]
}

# Visible frame of the main display: "x,y,w,h" (what maximize must equal).
visible_frame() {
    # Exact visible frame of the main display, in AX coordinates
    # (top-left origin, y down). Computed once per run.
    local vis="${TMPDIR:-/tmp}/wnav-verify-vis"
    if [ ! -x "$vis" ]; then
        cat > "$vis.swift" <<'SWIFT'
import AppKit
let v = NSScreen.screens[0].visibleFrame
let f = NSScreen.screens[0].frame
print("\(Int(v.origin.x)),\(Int(f.height - v.origin.y - v.height)),\(Int(v.width)),\(Int(v.height))")
SWIFT
        xcrun swiftc -O -o "$vis" "$vis.swift"
        rm -f "$vis.swift"
    fi
    "$vis"
}

# ------------------------------------------------------------- one cycle ----
run_cycle() {
    local cycle="$1"
    log "=== cycle $cycle/$loops ==="

    local version
    if [ "$no_install" = 1 ]; then
        version="running:$PROC"
        log "driving already-running process: $PROC"
    elif [ -n "$app_src" ]; then
        log "installing from bundle: $app_src"
        install_from_app "$app_src"
        version=$(installed_version)
    else
        [ -n "$dmg" ] || dmg=$(ls -t "$HOME"/Downloads/wnav-*.dmg 2>/dev/null | head -1)
        [ -n "$dmg" ] || die "no --dmg given and no wnav-*.dmg in ~/Downloads"
        log "installing from dmg: $dmg"
        install_from_dmg "$dmg"
        version=$(installed_version)
    fi
    log "version: $version"

    if [ "$no_install" != 1 ]; then open -g "$INSTALL_DIR"; fi
    wait_for_status_item || die "status item never appeared (app did not finish launching)"
    # The trust verdict is read AFTER the checks (below): tccd only logs a
    # denial once the app actually asks for Accessibility, so probing right
    # after launch races the event and reports a false "granted".

    local items
    items=$(osascript -e "tell application \"System Events\" to tell process \"$PROC\" to get name of every menu item of menu of menu bar item 1 of menu bar 2")
    case "$items" in
        *Left*Right*Preferences*) log "status menu ok" ;;
        *) die "status menu incomplete: $items" ;;
    esac

    # Target window: a real TextEdit document, frontmost, starting full-size.
    osascript -e 'tell application "TextEdit" to close every window saving no' 2>/dev/null || true
    osascript -e 'tell application "TextEdit" to activate' -e 'tell application "TextEdit" to make new document' >/dev/null
    osascript -e 'tell application "System Events" to tell process "TextEdit" to set position of window 1 to {0, 30}' -e 'tell application "System Events" to tell process "TextEdit" to set size of window 1 to {900, 600}' >/dev/null
    local vf
    vf=$(visible_frame)
    IFS=, read -r vx vy vw vh <<<"$vf"
    local half=$((vw / 2))
    log "visible frame: $vf"
    osascript -e 'tell application "TextEdit" to activate' >/dev/null
    sleep 0.6
    if ! ensure_focus; then
        log "INCONCLUSIVE: TextEdit could not be made frontmost ('$(target_focused)' holds focus)."
        log "            No keys were sent: a global hotkey would move THAT app's window."
    else
        log "keyboard hotkeys (real CGEvents) on TextEdit"
        # Each press_* secures and verifies focus itself, then records which
        # app was frontmost when the key landed (see `press`).
        press_left;     sleep 0.7; check "hotkey left"     "$vx,$vy,$half,$vh"
        press_right;    sleep 0.7; check "hotkey right"    "$((vx + vw - half)),$vy,$half,$vh"
        press_up;       sleep 0.7; check "hotkey top"      "$vx,$vy,$vw,$((vh / 2))"
        press_down;     sleep 0.7; check "hotkey bottom"  "$vx,$((vy + vh - vh / 2)),$vw,$((vh / 2))"
        press_maximize; sleep 0.7; check "hotkey maximize" "$vx,$vy,$vw,$vh"
    fi

    log "status menu"
    if ! ensure_focus; then
        log "INCONCLUSIVE: TextEdit could not be made frontmost before the menu check."
    else
        FOCUSED_AT_KEY="TextEdit"
        osascript -e "tell application \"System Events\" to tell process \"$PROC\" to click menu item \"Left\" of menu of menu bar item 1 of menu bar 2" >/dev/null
        sleep 0.8; check "menu Left" "$vx,$vy,$half,$vh"
    fi

    # Now the app has certainly made its Accessibility requests.
    TRUST=$(ax_trusted)
    log "accessibility trust for this build: $TRUST"

    # ok is only 1 when every check passed AND none were inconclusive: an
    # inconclusive run has not verified anything.
    local ok=0
    [ "$FAIL" -eq 0 ] && [ "$INCONCLUSIVE" -eq 0 ] && ok=1
    local failures_json="" f
    for f in "${FAILURES[@]}"; do
        [ -n "$f" ] || continue
        f=${f//\\/\\\\}; f=${f//\"/\\\"}
        failures_json="$failures_json\"$f\","
    done
    mkdir -p "$REPORT_DIR"
    printf '{"version":"%s","cycle":%s,"ok":%s,"accessibility":"%s","pass":%s,"fail":%s,"inconclusive":%s,"failures":[%s],"checks":[%s]}\n' \
        "$version" "$cycle" "$ok" "$TRUST" "$PASS" "$FAIL" "$INCONCLUSIVE" \
        "${failures_json%,}" "${RESULTS%,}" > "$REPORT_DIR/verify-$cycle.json"
    if fresh_install_denied; then
        log "CAUSE: Accessibility is not granted to this build, so NOTHING can move a window."
        log "       macOS stores the grant against the old binary's hash (cdhash), and a new"
        log "       build no longer matches it. Wnav itself is working: the same keys pass 6/6"
        log "       on a build that holds the grant."
        log "FIX:   System Settings → Privacy & Security → Accessibility"
        log "         1. select the stale 'Wnav' entry and remove it (minus button)"
        log "         2. add /Applications/Wnav.app again (plus button) and enable it"
        log "       Releases are now signed with an identifier-only requirement, so this is a"
        log "       one-time re-grant: later builds keep it. Then re-run this loop."
    fi
    [ "$ok" = 1 ] || return 1
}

build_key_helper
backup_config
RC=0
for n in $(seq 1 "$loops"); do
    PASS=0; FAIL=0; INCONCLUSIVE=0; FAILURES=(); RESULTS=""
    if ! run_cycle "$n"; then RC=1; fi
    log "cycle $n: $PASS passed, $FAIL failed, $INCONCLUSIVE inconclusive → $REPORT_DIR/verify-$n.json"
    if [ "$n" -lt "$loops" ] && [ "$loop_sleep" -gt 0 ]; then sleep "$loop_sleep"; fi
done
if [ "$RC" = 0 ]; then log "ALL CHECKS PASSED"; else log "FAILURES: ${FAILURES[*]:-see report}"; fi
exit "$RC"
