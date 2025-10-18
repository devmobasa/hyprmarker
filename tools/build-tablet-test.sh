#!/bin/bash
# Build hyprmarker with tablet support and package for testing

set -e

echo "🔨 Building hyprmarker with tablet support..."
cargo build --release --features tablet-input

echo ""
echo "📦 Creating test package..."
mkdir -p tablet-test-build
cp target/release/hyprmarker tablet-test-build/
cp docs/TABLET_TESTING.md tablet-test-build/
cp config.example.toml tablet-test-build/

# Create test config with tablet enabled
cat > tablet-test-build/test-config.toml << 'EOF'
# Test configuration with tablet support enabled
# Copy this to ~/.config/hyprmarker/config.toml

[drawing]
default_color = "red"
default_thickness = 3.0
default_font_size = 32.0

[tablet]
# ⚠️  IMPORTANT: Set this to true to enable tablet input
enabled = true
pressure_enabled = true
min_thickness = 1.0
max_thickness = 8.0

[ui]
show_status_bar = true
status_bar_position = "bottom-left"

[board]
enabled = true
default_mode = "transparent"
EOF

# Create test script
cat > tablet-test-build/test-tablet.sh << 'EOF'
#!/bin/bash
# Quick test script for tablet functionality

echo "🖊️  Tablet Test Script"
echo "===================="
echo ""
echo "This will run hyprmarker with detailed logging."
echo "Use Ctrl+C to stop, then check the tablet-test.log file."
echo ""
echo "Press Enter to start..."
read

RUST_LOG=hyprmarker=debug,info ./hyprmarker --active 2>&1 | tee tablet-test.log
EOF
chmod +x tablet-test-build/test-tablet.sh

# Create daemon test script
cat > tablet-test-build/test-daemon.sh << 'EOF'
#!/bin/bash
# Test daemon mode with logging

echo "🖊️  Tablet Daemon Test"
echo "====================="
echo ""
echo "This will run hyprmarker daemon with detailed logging."
echo "Use 'pkill -SIGUSR1 hyprmarker' to toggle the overlay."
echo "Use Ctrl+C to stop."
echo ""
echo "Press Enter to start..."
read

RUST_LOG=hyprmarker=debug,info ./hyprmarker --daemon 2>&1 | tee tablet-daemon.log
EOF
chmod +x tablet-test-build/test-daemon.sh

# Create installation instructions
cat > tablet-test-build/INSTALL.md << 'EOF'
# Installation Instructions for Testing

## Quick Start

1. **Install the binary:**
   ```bash
   mkdir -p ~/.local/bin
   cp hyprmarker ~/.local/bin/
   chmod +x ~/.local/bin/hyprmarker
   ```

2. **Set up config:**
   ```bash
   mkdir -p ~/.config/hyprmarker
   cp test-config.toml ~/.config/hyprmarker/config.toml
   ```

3. **Run test:**
   ```bash
   ./test-tablet.sh
   ```

4. **Follow testing guide:**
   - Read `TABLET_TESTING.md` for detailed testing steps
   - Try drawing with your stylus
   - Check the generated `tablet-test.log` file
   - Look for "TABLET FOUND" and stylus event messages

## What to Report

After testing, send back:

1. The `tablet-test.log` file
2. Output of: `hyprctl devices | grep -A5 Tablets`
3. Answer the checklist questions in TABLET_TESTING.md
4. Any screenshots or videos of issues

## Troubleshooting

If tablet isn't detected:
- Make sure `hyprctl devices` shows your tablet
- Verify config has `enabled = true` under `[tablet]`
- Check for "TABLET NOT FOUND" vs "TABLET FOUND" in logs

If you need help, send the log file and system info!
EOF

echo ""
echo "✅ Test package created in: tablet-test-build/"
echo ""
echo "📁 Package contents:"
ls -lh tablet-test-build/
echo ""
echo "📤 Next steps:"
echo "   1. Compress: tar -czf tablet-test-build.tar.gz tablet-test-build/"
echo "   2. Send to your friend"
echo "   3. They should follow instructions in tablet-test-build/INSTALL.md"
echo ""
