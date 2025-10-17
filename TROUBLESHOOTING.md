# Troubleshooting Guide

Quick solutions for common issues with Overlay Native.

## 🔍 Quick Diagnosis Tool

Before diving into troubleshooting, run the diagnostic tool:

```bash
cargo run --bin test_emotes
```

This tests each emote provider (BTTV, FFZ, 7TV) and identifies problems.

---

## Common Startup Errors

### ❌ Error: "Error de red: error decoding response body"

**Cause:** One of the emote providers (BTTV, FFZ, 7TV) is returning unexpected data.

**Solution:**

1. Run the diagnostic tool to identify which provider is failing:
   ```bash
   cargo run --bin test_emotes
   ```

2. Check the output to see which provider failed

3. If a provider consistently fails, you can disable it in `config.json`:
   ```json
   {
     "emotes": {
       "enable_bttv": true,
       "enable_ffz": true,
       "enable_7tv": true
     }
   }
   ```

4. Set the failing provider to `false` and restart the application

---

### ❌ Network Connectivity Issues

**Symptoms:**
- `Failed to fetch <URL>`
- `connection error`
- `timed out`

**Solutions:**

1. **Check internet connection** - Verify you can access:
   - https://api.betterttv.net/3/cached/emotes/global
   - https://api.frankerfacez.com/v1/set/global
   - https://7tv.io/v3/emote-sets/global

2. **Check firewall/proxy** - Ensure Rust applications can make HTTPS requests

3. **Wait and retry** - The app now has automatic retry logic with exponential backoff

---

### ❌ Rate Limiting (HTTP 429)

**Symptoms:**
- `HTTP 429 from <URL>`
- `Too Many Requests`

**Solutions:**

1. Wait 5-10 minutes before trying again
2. The application will automatically retry with increasing delays
3. If it persists, an API provider may have changed their rate limits

---

### ⚠️ Application Starts But Some Providers Failed

**What you'll see:**
```
⚠️  Some providers failed:
   - bttv: Failed to load emotes...
📊 Total emotes loaded: 88
```

**This is OK!** The application will continue with the working providers.

**To fix:**
1. Check which provider failed in the logs
2. Run `cargo run --bin test_emotes` to diagnose
3. Wait and try restarting (might be temporary)
4. Disable the failing provider in config if needed

---

## Running the Application

### Default Run
```bash
cargo run
```

### With Diagnostic First
```bash
# 1. Test all providers
cargo run --bin test_emotes

# 2. If all pass, run the main app
cargo run
```

### Specify Binary Explicitly
```bash
cargo run --bin overlay-native
```

---

## Advanced Debugging

### Enable Detailed Logs

The application already provides detailed emote loading information:
```
📥 Loading bttv global emotes...
✅ Loaded 65 emotes from bttv
📥 Loading ffz global emotes...
⚠️ Failed to load ffz emotes: <detailed error>
```

### Test Individual Providers with Unit Tests

```bash
# Test BTTV
cargo test --lib test_bttv_global_emotes

# Test FFZ  
cargo test --lib test_ffz_global_emotes

# Test 7TV
cargo test --lib test_7tv_global_emotes

# Test all providers
cargo test --lib test_all_providers_individually -- --nocapture
```

### Manually Check API Endpoints

```bash
# Windows PowerShell
Invoke-WebRequest -Uri "https://api.betterttv.net/3/cached/emotes/global"

# Linux/Mac/Git Bash
curl https://api.betterttv.net/3/cached/emotes/global
```

---

## Configuration Issues

### Missing or Invalid config.json

The application will use defaults if `config.json` is missing or invalid:
```
Error loading config: ..., using defaults
```

**Solution:** Create a valid `config.json` or let it use defaults.

### Invalid Platform Credentials

If you see authentication errors for Twitch/YouTube/etc.:
1. Check your credentials in `config.json`
2. Verify tokens haven't expired
3. Check platform-specific requirements

---

## Platform-Specific Issues

### Linux: GTK Errors

**Symptoms:**
- `Cannot load styles file`
- `Cannot get main screen for styling`

**Solutions:**
1. Ensure GTK 3.0+ is installed: `sudo apt install libgtk-3-dev`
2. Check X11 is running: `echo $DISPLAY`
3. Try running with Wayland/XWayland

### Windows: Monitor Detection

**Symptoms:**
- Window appears on wrong monitor
- Incorrect overlay positioning

**Solutions:**
1. Check monitor geometry in startup logs
2. Adjust `monitor_margin` in `config.json`
3. Verify display scaling settings

---

## Getting More Help

If issues persist:

1. **Run the diagnostic tool** and save output:
   ```bash
   cargo run --bin test_emotes > diagnostic.txt
   ```

2. **Check logs** for detailed error messages

3. **Report the issue** with:
   - Output from `test_emotes`
   - Full error messages from main application
   - Your configuration (redact sensitive info)
   - OS and Rust version: `rustc --version`

4. **See full debugging guide**: [docs/DEBUGGING.md](docs/DEBUGGING.md)

---

## Quick Fixes Summary

| Issue | Quick Fix |
|-------|-----------|
| Network error on startup | Run `cargo run --bin test_emotes` |
| One provider failing | Disable it in `config.json` |
| Rate limited | Wait 5-10 minutes |
| All providers fail | Check internet/firewall |
| GTK errors (Linux) | Install `libgtk-3-dev` |
| Wrong monitor (Windows) | Adjust `monitor_margin` in config |

---

## Success Indicators

When everything works, you should see:

```
🚀 Starting Overlay Native...
✅ Platform twitch initialized
🔄 Preloading global emotes...
   📥 Loading bttv global emotes...
   ✅ Loaded 65 emotes from bttv
   📥 Loading ffz global emotes...
   ✅ Loaded 23 emotes from ffz
   📥 Loading 7tv global emotes...
   ✅ Loaded 43 emotes from 7tv
📊 Total emotes loaded: 131
✅ Global emotes preloaded
✅ Connected to <channel> on <platform>
```

If you see this, everything is working correctly! 🎉