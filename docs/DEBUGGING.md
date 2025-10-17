# Debugging Guide for Overlay Native

This guide helps you diagnose and fix issues with the Overlay Native application, with a focus on emote provider problems.

## Table of Contents

- [Quick Diagnosis](#quick-diagnosis)
- [Using the Emote Provider Test Tool](#using-the-emote-provider-test-tool)
- [Common Issues](#common-issues)
- [Understanding Error Messages](#understanding-error-messages)
- [Debugging Steps](#debugging-steps)

---

## Quick Diagnosis

If the application crashes with an error like:

```
Error: Error de red: error decoding response body
```

Run the diagnostic tool first:

```bash
cargo run --bin test_emotes
```

This will test each emote provider individually and identify which one is failing.

---

## Using the Emote Provider Test Tool

### What It Does

The `test_emotes` binary tests each emote provider (BTTV, FFZ, 7TV, Twitch) independently to identify issues before running the main application.

### Running the Tool

```bash
cargo run --bin test_emotes
```

### Example Output

#### ✅ Success (All providers working):

```
🔍 Overlay Native - Emote Provider Diagnostic Tool

📦 Testing BTTV Provider...
   🔄 Fetching global emotes...
   ✅ Successfully loaded 65 emotes
   📝 Sample emotes:
      - :tf: (ID: 54fa8f1401e468494b85b537)
      - CiGrip (ID: 54fa8fce01e468494b85b53c)

📦 Testing FFZ Provider...
   ✅ Successfully loaded 23 emotes

📦 Testing 7TV Provider...
   ✅ Successfully loaded 43 emotes

📊 Summary:
   ✅ BTTV: 65 emotes loaded
   ✅ FFZ: 23 emotes loaded
   ✅ 7TV: 43 emotes loaded
   ✅ Twitch: OK (Auth required)

✅ All providers working correctly!
```

#### ❌ Failure (Provider issue detected):

```
📦 Testing BTTV Provider...
   ⚠️  Attempt 1/3 failed for https://api.betterttv.net/3/cached/emotes/global
   ❌ Failed: Error de red: Failed to parse JSON from https://api...

📊 Summary:
   ❌ BTTV: FAILED
      Error: Error de red: Failed to parse JSON...
   ✅ FFZ: 23 emotes loaded
   ✅ 7TV: 43 emotes loaded

⚠️  Some providers failed. Common issues:
   1. Network connectivity problems
   2. API endpoint changes
   3. Rate limiting
   4. Firewall/proxy blocking requests
```

---

## Common Issues

### 1. Network Connectivity Problems

**Symptoms:**
- `Failed to fetch <URL>: connection error`
- `timed out`

**Solutions:**
- Check your internet connection
- Verify you can access the API URLs in your browser:
  - BTTV: https://api.betterttv.net/3/cached/emotes/global
  - FFZ: https://api.frankerfacez.com/v1/set/global
  - 7TV: https://7tv.io/v3/emote-sets/global
- Check firewall/proxy settings

### 2. JSON Parsing Errors

**Symptoms:**
- `error decoding response body`
- `Failed to parse JSON from <URL>`

**Solutions:**
- API structure may have changed
- Run the test tool to see which provider is affected
- Check the API endpoint directly in your browser to see the JSON structure
- The error message now includes the URL, making it easier to identify which API changed

### 3. Rate Limiting

**Symptoms:**
- `HTTP 429` errors
- `Too Many Requests`

**Solutions:**
- Wait a few minutes before trying again
- The application now includes automatic retry logic with exponential backoff
- Retries happen automatically (3 attempts with increasing delays)

### 4. API Endpoint Changes

**Symptoms:**
- `HTTP 404` errors
- Persistent JSON parsing errors

**Solutions:**
- Check if the API provider has updated their endpoints
- Look at the error message for the full URL that failed
- Report the issue if it's a breaking change in the API

---

## Understanding Error Messages

### Improved Error Messages

The application now provides detailed error messages:

```
Error de red: Failed to parse JSON from https://api.betterttv.net/3/cached/emotes/global: error decoding response body
```

This tells you:
- **Type**: Network error (`Error de red`)
- **Action**: Failed to parse JSON
- **URL**: The exact API endpoint that failed
- **Reason**: What went wrong

### HTTP Status Codes in Errors

```
Error de red: HTTP 403 from https://...: Forbidden
```

- **200-299**: Success (shouldn't see errors)
- **400**: Bad Request - API parameters issue
- **403**: Forbidden - Authentication/authorization issue
- **404**: Not Found - Endpoint doesn't exist
- **429**: Too Many Requests - Rate limited
- **500-599**: Server Error - API provider issue

---

## Debugging Steps

### Step 1: Test Individual Providers

```bash
cargo run --bin test_emotes
```

This will identify which provider(s) are failing.

### Step 2: Check Network Access

Try accessing the failing API directly:

```bash
# Windows PowerShell
Invoke-WebRequest -Uri "https://api.betterttv.net/3/cached/emotes/global"

# Linux/Mac
curl https://api.betterttv.net/3/cached/emotes/global
```

### Step 3: Review Application Logs

The main application now provides detailed logging:

```
🔄 Preloading global emotes...
   📥 Loading bttv global emotes...
   ✅ Loaded 65 emotes from bttv
   📥 Loading ffz global emotes...
   ⚠️  Failed to load ffz emotes: Error de red...
```

This shows:
- Which providers are being tested
- How many emotes were loaded
- Which providers failed and why

### Step 4: Retry with Automatic Backoff

The application now automatically retries failed requests:

```
⚠️  Attempt 1/3 failed for <URL>: <error>. Retrying...
⚠️  Attempt 2/3 failed for <URL>: <error>. Retrying...
```

- 3 total attempts per request
- Exponential backoff (500ms, 1s, 2s delays)
- Helps with transient network issues

### Step 5: Run with Partial Providers

Even if some providers fail, the application will continue with the working ones:

```
📊 Total emotes loaded: 88
⚠️  Some providers failed:
   - bttv: Failed to parse JSON...
✅ Global emotes preloaded (with warnings)
```

The app won't crash - it continues with available providers.

---

## Testing Changes

### Running Tests

```bash
# Test all emote providers
cargo test --lib emotes::providers::tests

# Test a specific provider
cargo test --lib test_bttv_global_emotes

# Test all providers individually (integration-style)
cargo test --lib test_all_providers_individually
```

### Available Tests

- `test_bttv_global_emotes` - Tests BTTV global emotes API
- `test_ffz_global_emotes` - Tests FFZ global emotes API
- `test_7tv_global_emotes` - Tests 7TV global emotes API
- `test_twitch_provider` - Verifies Twitch provider initialization
- `test_api_client` - Tests the HTTP client with a simple request
- `test_all_providers_individually` - Comprehensive test of all providers

### Adding New Tests

When adding a new emote provider, create tests following this pattern:

```rust
#[tokio::test]
async fn test_my_provider_global_emotes() {
    let provider = MyEmoteProvider::new();
    match provider.get_global_emotes().await {
        Ok(emotes) => {
            println!("✅ MyProvider: Loaded {} global emotes", emotes.len());
            assert!(!emotes.is_empty(), "MyProvider should have global emotes");
        }
        Err(e) => {
            panic!("❌ MyProvider failed: {}", e);
        }
    }
}
```

---

## Improvements Made

### 1. Enhanced Error Messages

- Errors now include the failing URL
- HTTP status codes are shown
- Response bodies are included when possible

### 2. Retry Logic

- Automatic retries for transient failures
- Exponential backoff to avoid hammering APIs
- Configurable number of retries

### 3. Graceful Degradation

- Application continues if some providers fail
- Per-provider error logging
- Summary of successful vs failed providers

### 4. Diagnostic Tool

- Standalone binary for testing providers
- No UI dependencies (works on headless systems)
- Clear visual feedback

### 5. Comprehensive Tests

- Unit tests for each provider
- Integration-style tests
- Easy to run and interpret

---

## Getting Help

If you've followed these debugging steps and still have issues:

1. **Check the logs** - Look for specific error messages
2. **Run the test tool** - `cargo run --bin test_emotes`
3. **Test API endpoints manually** - Verify they're accessible
4. **Check for API updates** - Providers sometimes change their APIs
5. **Report the issue** - Include:
   - Output from `test_emotes`
   - Full error messages
   - Which providers are failing
   - Your network environment (proxy, firewall, etc.)

---

## Configuration

You can configure emote providers in `config.json`:

```json
{
  "emotes": {
    "cache_enabled": true,
    "cache_ttl_hours": 24,
    "enable_bttv": true,
    "enable_ffz": true,
    "enable_7tv": true,
    "max_emotes_per_message": 50
  }
}
```

To disable a failing provider temporarily:

```json
{
  "emotes": {
    "enable_bttv": false  // Disable BTTV if it's causing issues
  }
}
```

---

## Performance Tips

1. **Cache is your friend** - Keep `cache_enabled: true`
2. **Adjust TTL** - Lower `cache_ttl_hours` if you want fresher emotes
3. **Disable unused providers** - Turn off providers you don't need
4. **Monitor startup time** - The test tool shows how long each provider takes

---

## Version History

- **v0.1.1** - Added retry logic, improved error messages, created test tool
- **v0.1.0** - Initial release

---

For more information, see the main [README.md](../README.md).