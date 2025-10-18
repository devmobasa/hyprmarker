#!/bin/bash
# Check if the system environment is ready for tablet testing

echo "🔍 Tablet Environment Check"
echo "============================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_pass() {
    echo -e "${GREEN}✓${NC} $1"
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Check 1: Wayland session
echo "1. Checking Wayland environment..."
if [ -n "$WAYLAND_DISPLAY" ]; then
    check_pass "Wayland session detected ($WAYLAND_DISPLAY)"
else
    check_fail "Not running Wayland (WAYLAND_DISPLAY not set)"
    echo "   Tablet support requires Wayland compositor"
fi
echo ""

# Check 2: Hyprland version
echo "2. Checking compositor..."
if command -v hyprctl &> /dev/null; then
    VERSION=$(hyprctl version | head -n1)
    check_pass "Hyprland found: $VERSION"

    # Check for tablet devices
    echo "   Checking for tablet devices..."
    TABLETS=$(hyprctl devices 2>/dev/null | grep -A10 "Tablets:" | grep -v "Tablets:" | head -n5)
    if [ -n "$TABLETS" ]; then
        check_pass "Tablet devices found:"
        echo "$TABLETS" | while read line; do
            if [ -n "$line" ]; then
                echo "      • $line"
            fi
        done
    else
        check_warn "No tablet devices detected by Hyprland"
        echo "   Connect your tablet and run this script again"
    fi
else
    check_fail "hyprctl not found (are you using Hyprland?)"
fi
echo ""

# Check 3: libinput devices
echo "3. Checking libinput devices..."
if command -v libinput &> /dev/null; then
    TABLET_DEVICES=$(libinput list-devices 2>/dev/null | grep -i "tablet\|wacom\|pen" || true)
    if [ -n "$TABLET_DEVICES" ]; then
        check_pass "Tablet-related input devices found"
        echo "$TABLET_DEVICES" | head -n10
    else
        check_warn "No tablet devices in libinput"
    fi
else
    check_warn "libinput command not found"
fi
echo ""

# Check 4: Kernel modules
echo "4. Checking kernel support..."
if lsmod | grep -q wacom; then
    check_pass "Wacom kernel module loaded"
else
    check_warn "Wacom kernel module not loaded (might be built-in or not needed)"
fi

# Check USB devices
WACOM_USB=$(lsusb | grep -i wacom || true)
if [ -n "$WACOM_USB" ]; then
    check_pass "Wacom USB device detected:"
    echo "   $WACOM_USB"
else
    check_warn "No Wacom USB devices found (might be Bluetooth or other connection)"
fi
echo ""

# Check 5: Config file
echo "5. Checking hyprmarker configuration..."
CONFIG_PATH="$HOME/.config/hyprmarker/config.toml"
if [ -f "$CONFIG_PATH" ]; then
    check_pass "Config file exists: $CONFIG_PATH"

    # Check if tablet is enabled
    if grep -q "^\[tablet\]" "$CONFIG_PATH"; then
        if grep -q "^enabled = true" "$CONFIG_PATH"; then
            check_pass "Tablet support enabled in config"
        else
            check_warn "Tablet section exists but enabled = false"
            echo "   Set 'enabled = true' in [tablet] section"
        fi
    else
        check_warn "No [tablet] section in config"
        echo "   Add tablet configuration section"
    fi
else
    check_warn "Config file not found at $CONFIG_PATH"
    echo "   You'll need to create this before testing"
fi
echo ""

# Check 6: Build features
echo "6. Checking hyprmarker binary..."
if [ -f "./target/release/hyprmarker" ]; then
    check_pass "Release binary exists"

    # Try to determine if tablet feature is compiled (rough check)
    if strings ./target/release/hyprmarker | grep -q "zwp_tablet"; then
        check_pass "Binary appears to have tablet support compiled in"
    else
        check_warn "Binary might not have tablet support"
        echo "   Build with: cargo build --release --features tablet-input"
    fi
elif [ -f "$HOME/.local/bin/hyprmarker" ]; then
    check_pass "Installed binary found at ~/.local/bin/hyprmarker"

    if strings "$HOME/.local/bin/hyprmarker" | grep -q "zwp_tablet"; then
        check_pass "Installed binary has tablet support"
    else
        check_warn "Installed binary might not have tablet support"
    fi
else
    check_fail "hyprmarker binary not found"
    echo "   Build with: cargo build --release --features tablet-input"
fi
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Summary:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -n "$TABLETS" ]; then
    echo -e "${GREEN}✓ Ready to test!${NC} Tablet detected and environment looks good."
    echo ""
    echo "Next steps:"
    echo "  1. Build with tablet support: cargo build --release --features tablet-input"
    echo "  2. Configure: Edit ~/.config/hyprmarker/config.toml"
    echo "  3. Test: RUST_LOG=info ./target/release/hyprmarker --active"
elif [ -n "$WACOM_USB" ]; then
    echo -e "${YELLOW}⚠ Tablet hardware detected but not by Hyprland${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  • Restart Hyprland"
    echo "  • Check tablet is not claimed by another app"
    echo "  • Update Hyprland to latest version"
else
    echo -e "${YELLOW}⚠ No tablet detected${NC}"
    echo ""
    echo "Make sure your tablet is:"
    echo "  • Connected (USB/Bluetooth)"
    echo "  • Powered on"
    echo "  • Drivers installed (if needed)"
fi
echo ""
