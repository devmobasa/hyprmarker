# Wacom Tablet Testing Guide

This guide helps test and debug the Wacom tablet implementation remotely.

## Prerequisites

1. **Build with tablet support:**
   ```bash
   cargo build --release --features tablet-input
   ```

2. **Enable in config** (`~/.config/hyprmarker/config.toml`):
   ```toml
   [tablet]
   enabled = true
   pressure_enabled = true
   min_thickness = 1.0
   max_thickness = 8.0
   ```

## Step-by-Step Testing

### Step 1: Verify Tablet is Detected by System

Before testing hyprmarker, confirm your tablet works with Hyprland:

```bash
# Check if Hyprland sees the tablet
hyprctl devices

# You should see output like:
# Tablets:
#   Wacom Movink Pen
```

**If tablet NOT listed:** Your tablet isn't being recognized by the compositor. Fix this first:
- Check `libinput list-devices` to see if the tablet is detected at all
- Verify tablet drivers are installed
- Check `dmesg | grep -i wacom` for kernel messages

### Step 2: Run hyprmarker with Detailed Logging

**Option A: Active Mode (Quick Test)**
```bash
RUST_LOG=hyprmarker=debug,info ./target/release/hyprmarker --active 2>&1 | tee tablet-test.log
```

**Option B: Daemon Mode**
```bash
# Stop existing daemon if running
systemctl --user stop hyprmarker.service

# Run manually with logging
RUST_LOG=hyprmarker=debug,info ./target/release/hyprmarker --daemon 2>&1 | tee tablet-test.log
```

### Step 3: Look for Detection Messages

In the logs, you should see:

```
✅ GOOD - Tablet compiled and enabled:
[INFO] Tablet feature: compiled=yes, runtime_enabled=true
[INFO] Bound zwp_tablet_manager_v2

✅ GOOD - Tablet device found:
[INFO] 🖊️  TABLET DEVICE DETECTED
[INFO] TABLET FOUND - Total devices: 1

✅ GOOD - Stylus/pen tool found:
[INFO] 🖊️  TABLET TOOL DETECTED (pen/stylus)
[INFO] TABLET FOUND - Total tools: 1
```

**If you see:**
```
❌ BAD:
[WARN] Tablet protocol not available: ...
[INFO] TABLET NOT FOUND
```
→ The compositor doesn't support tablet protocol or tablet isn't connected

### Step 4: Test Stylus Input

With hyprmarker overlay active:

1. **Hover Test:**
   - Hover stylus over screen (don't touch)
   - **Expected log:** `[INFO] ✏️  Stylus ENTERED overlay surface`
   - Move stylus away
   - **Expected log:** `[INFO] ✏️  Stylus LEFT overlay surface`

2. **Drawing Test:**
   - Touch stylus to screen
   - **Expected log:** `[INFO] ✏️  Stylus DOWN at (x, y)`
   - Draw a stroke
   - **Expected log (debug):** Multiple `[DEBUG] Stylus motion: (x, y)`
   - Lift stylus
   - **Expected log:** `[INFO] ✏️  Stylus UP at (x, y)`

3. **Pressure Test:**
   - Draw with varying pressure
   - **Expected log (debug):**
     ```
     [DEBUG] Stylus pressure: 0.23 (raw: 15000/65535)
     [DEBUG] Pressure 0.23 → thickness 2.6px (range: 1.0-8.0)
     ```

### Step 5: Verify Drawing Works

- **Draw multiple strokes** with the stylus
- Strokes should appear on screen
- Try varying pressure - thickness should change between strokes
- Try different colors (press R, G, B keys)

### Step 6: Test Mouse Still Works

- **Draw with mouse** - should work normally
- **Switch between stylus and mouse** - both should work

## Common Issues & Solutions

### Issue: "TABLET NOT FOUND" in logs

**Diagnosis:**
```bash
# Check what protocols compositor supports
WAYLAND_DEBUG=1 hyprmarker --active 2>&1 | grep tablet
```

**Solutions:**
1. Update Hyprland to latest version (tablet protocol added in v0.20+)
2. Verify tablet is plugged in and detected by system
3. Try rebooting (sometimes helps with USB tablet detection)

### Issue: Tablet detected but no input events

**Check logs for:**
- "Stylus ENTERED" messages → stylus is hovering but not triggering
- "Stylus DOWN" messages → stylus is pressing

**If no proximity events:**
- Tablet might be configured to different output
- Try mapping tablet to current monitor: `xsetwacom` (if available)

### Issue: Drawing works but pressure doesn't

**Check config:**
```toml
[tablet]
pressure_enabled = true  # ← Must be true
```

**Check logs for:**
```
[DEBUG] Stylus pressure: ...
```

If you see pressure events but thickness doesn't change:
- Check min/max thickness values aren't the same
- Verify `RUST_LOG` includes `debug` level

### Issue: Coordinates are wrong (drawing in wrong place)

This can happen with:
- Multi-monitor setups
- Scaled displays (HiDPI)
- Tablet mapped to different screen

**Debug:**
- Compare logged coordinates with actual touch position
- Try on single monitor first
- Check Hyprland tablet mapping configuration

## Collecting Debug Information

If things don't work, send this information:

1. **System info:**
   ```bash
   hyprctl version
   hyprctl devices | grep -A5 "Tablets"
   uname -a
   ```

2. **Log file:**
   ```bash
   RUST_LOG=hyprmarker=trace ./target/release/hyprmarker --active 2>&1 | tee full-debug.log
   # Try using the tablet, then send full-debug.log
   ```

3. **Config file:**
   ```bash
   cat ~/.config/hyprmarker/config.toml
   ```

4. **Build info:**
   ```bash
   cargo --version
   ./target/release/hyprmarker --version
   # Confirm it was built with --features tablet-input
   ```

## Expected Behavior vs Actual

### ✅ What SHOULD work:
- Stylus draws when touching screen
- Hover detection (stylus near screen)
- Pressure affects thickness of **next** stroke
- Stylus and mouse both work independently

### ⚠️ Known Limitations:
- Pressure creates uniform-width strokes (thickness changes between strokes, not during)
- No tilt/rotation support yet
- No eraser end support yet
- Each stroke has single thickness (not variable-width like Photoshop)

### ❌ What should NOT happen:
- Tablet input interferes with mouse
- Crashes when using stylus
- Drawing appears at wrong coordinates

## Quick Validation Checklist

Send back answers to these:

- [ ] `hyprctl devices` shows tablet?
- [ ] Logs show "TABLET FOUND"?
- [ ] Logs show "Stylus ENTERED" when hovering?
- [ ] Logs show "Stylus DOWN/UP" when drawing?
- [ ] Logs show pressure values?
- [ ] Can draw strokes with stylus?
- [ ] Thickness changes with pressure?
- [ ] Mouse still works?
- [ ] Any crashes or errors?

## Advanced: Wayland Protocol Debugging

If you want to see raw Wayland events:

```bash
WAYLAND_DEBUG=1 RUST_LOG=debug ./target/release/hyprmarker --active 2>&1 | tee wayland-debug.log
# Warning: VERY verbose output
# Look for "zwp_tablet" events
```

## Contact

When reporting issues, include:
- Which checklist items failed
- Relevant log excerpts (especially ERROR/WARN lines)
- System info from "Collecting Debug Information" section

Good luck testing! 🖊️
