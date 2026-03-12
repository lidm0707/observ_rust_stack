# Glitchtip DSN Setup with Rust Application

## Overview

This guide provides step-by-step instructions for configuring Glitchtip (Sentry-compatible error tracking) with the Rust Actix-web application using Sentry SDK.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Understanding the Integration](#understanding-the-integration)
3. [Getting DSN from Glitchtip](#getting-dsn-from-glitchtip)
4. [Configuring Environment Variables](#configuring-environment-variables)
5. [How Rust Reads Configuration](#how-rust-reads-configuration)
6. [Testing the Setup](#testing-the-setup)
7. [Troubleshooting](#troubleshooting)
8. [Best Practices](#best-practices)
9. [Advanced Configuration](#advanced-configuration)

## Prerequisites

Before starting, ensure you have:

- ✅ Glitchtip service running (`./start.sh`)
- ✅ Glitchtip admin account created (`docker exec -it glitchtip_web python manage.py createsuperuser`)
- ✅ Actix-web application running
- ✅ Access to `actix-app/.env` file

## Understanding the Integration

### How It Works

```
┌─────────────────┐
│   Rust App      │
│  (actix-web)   │
└────────┬────────┘
         │
         │ Error occurs
         │
         ├────────────────────┬─────────────────────┐
         │                    │                     │
         ▼                    ▼                     ▼
┌──────────────┐   ┌──────────────┐    ┌──────────────┐
│   OpenObserve │   │   Glitchtip  │    │   OpenObserve │
│    (Logs)     │   │   (Errors)   │    │  (Metrics)    │
└──────────────┘   └──────────────┘    └──────────────┘
```

### Error Flow

1. **Error Occurs**: In Rust application (e.g., `/trigger-error` endpoint)
2. **Sentry SDK Captures**: Captures stack trace and context
3. **Sends to Glitchtip**: Via HTTP using SENTRY_DSN
4. **Glitchtip Processes**: Groups similar errors, creates issues
5. **Dashboard Display**: Shows in Glitchtip web UI

## Getting DSN from Glitchtip

### Step 1: Create Organization

1. Navigate to http://localhost:8000
2. Sign in with your admin credentials
3. Click "Create Organization" (if prompted)
4. Enter organization name (e.g., `my-company`)
5. Click "Create"

### Step 2: Create Project

1. Navigate to your organization
2. Click "Create Project"
3. Enter project name (e.g., `rust-actix-app`)
4. Select platform: `Rust` or `Other`
5. Click "Create Project"

### Step 3: Get DSN

**Method A: From Project Dashboard**

1. Go to your newly created project
2. Look for "Client Keys (DSN)" section
3. Click "Show DSN" or copy the DSN directly

**Method B: From Project Settings**

1. Go to Project → Settings
2. Click on "Client Keys (DSN)" or "Integration"
3. Copy the DSN

### DSN Format

A typical DSN looks like:

```
http://<public-dsn>@<glitchtip-host>/<project-id>
```

**Example:**
```
http://8d7f6c5e4d3c2b1a@localhost:8000/1234567890
```

**Components:**
- `http://` - Protocol (can be https for production)
- `8d7f6c5e4d3c2b1a` - Public DSN / Public Key
- `localhost:8000` - Glitchtip host and port
- `1234567890` - Project ID

## Configuring Environment Variables

### File Location

`actix-app/.env`

### Required Variables

```bash
# Glitchtip/Sentry Configuration
SENTRY_DSN=http://8d7f6c5e4d3c2b1a@localhost:8000/1234567890
SENTRY_ENVIRONMENT=development
SENTRY_RELEASE=actix-openobserve@0.1.0
SENTRY_SAMPLE_RATE=1.0
```

### Variable Explanations

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SENTRY_DSN` | ✅ Yes | None | Data Source Name from Glitchtip project |
| `SENTRY_ENVIRONMENT` | No | `development` | Environment name (development, staging, production) |
| `SENTRY_RELEASE` | No | `actix-openobserve@<version>` | Release identifier for tracking |
| `SENTRY_SAMPLE_RATE` | No | `1.0` | Error sampling rate (0.0 to 1.0) |

### Complete .env Example

```bash
# ========================================
# OpenObserve Configuration
# ========================================
OPENOBSERVE_HTTP_ENDPOINT=http://localhost:5080
OPENOBSERVE_ORG=default
OPENOBSERVE_STREAM=actix_demo
OPENOBSERVE_USER=admin@example.com
OPENOBSERVE_PASS=Complexpass#123

# ========================================
# Glitchtip/Sentry Configuration
# ========================================
# Get DSN from: http://localhost:8000 → Project → Settings → Client Keys
SENTRY_DSN=http://8d7f6c5e4d3c2b1a@localhost:8000/1234567890

# Environment: development, staging, or production
SENTRY_ENVIRONMENT=development

# Release: Optional but recommended for release tracking
SENTRY_RELEASE=actix-openobserve@0.1.0

# Sample rate: 1.0 = 100% errors, 0.1 = 10% errors (useful for high-traffic apps)
SENTRY_SAMPLE_RATE=1.0
```

### Different Environments

**Development:**
```bash
SENTRY_DSN=http://8d7f6c5e4d3c2b1a@localhost:8000/1234567890
SENTRY_ENVIRONMENT=development
SENTRY_RELEASE=actix-openobserve@0.1.0-dev
SENTRY_SAMPLE_RATE=1.0
```

**Staging:**
```bash
SENTRY_DSN=http://abcdef123456@staging.example.com/987654321
SENTRY_ENVIRONMENT=staging
SENTRY_RELEASE=actix-openobserve@0.1.0-rc1
SENTRY_SAMPLE_RATE=0.5
```

**Production:**
```bash
SENTRY_DSN=https://xyz789@glitchtip.example.com/111222333
SENTRY_ENVIRONMENT=production
SENTRY_RELEASE=actix-openobserve@0.1.0
SENTRY_SAMPLE_RATE=0.1
```

## How Rust Reads Configuration

### Code Location

File: `actix-app/src/main.rs`

### SentryConfig Struct (Lines 35-48)

```rust
#[derive(Clone)]
struct SentryConfig {
    dsn: String,
    environment: String,
    release: String,
    sample_rate: f32,
}

impl SentryConfig {
    fn from_env() -> Self {
        let dsn = std::env::var("SENTRY_DSN")
            .unwrap_or_else(|_| "http://8d7f6c5e4d3c2b1a@localhost:8000/1".into());
        let environment =
            std::env::var("SENTRY_ENVIRONMENT").unwrap_or_else(|_| "development".into());
        let release = std::env::var("SENTRY_RELEASE")
            .unwrap_or_else(|_| format!("actix-openobserve@{}", env!("CARGO_PKG_VERSION")));
        let sample_rate = std::env::var("SENTRY_SAMPLE_RATE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);
        SentryConfig {
            dsn,
            environment,
            release,
            sample_rate,
        }
    }
}
```

### Initialization Function (Lines 404-424)

```rust
fn init_sentry(cfg: &SentryConfig) -> anyhow::Result<sentry::ClientInitGuard> {
    let dsn = Dsn::from_str(&cfg.dsn)
        .map_err(|e| anyhow::anyhow!("Invalid Sentry DSN: {}", e))?;

    let client = sentry::init((
        dsn,
        ClientOptions {
            release: Some(cfg.release.clone().into()),
            environment: Some(cfg.environment.clone().into()),
            sample_rate: cfg.sample_rate,
            attach_stacktrace: true,
            send_default_pii: false,
            traces_sample_rate: 0.1,
            ..Default::default()
        },
    ));

    sentry::configure_scope(|scope| {
        scope.set_tag("service.name", "actix-openobserve");
        scope.set_tag("service.version", env!("CARGO_PKG_VERSION"));
        scope.set_tag("deployment.environment", &cfg.environment);
        scope.set_extra("http_endpoint", cfg.dsn.clone().into());
    });

    Ok(client)
}
```

### Usage in Main Function (Line 440)

```rust
let sentry_cfg = SentryConfig::from_env();
let _sentry_guard = init_sentry(&sentry_cfg)?;
```

### Error Capture in Handler (Lines 242-244)

```rust
// Capture error in Sentry/Glitchtip
let event_id = sentry::capture_message(&e.to_string(), sentry::Level::Error);
info!(event_id = %event_id, "Error captured in Sentry/Glitchtip");
```

### Configuration Flow

```
1. .env file
   ↓
2. Environment variables (loaded by dotenvy)
   ↓
3. SentryConfig::from_env() reads variables
   ↓
4. init_sentry() initializes Sentry client
   ↓
5. Errors are captured and sent to Glitchtip
```

## Testing the Setup

### Step 1: Update .env File

```bash
cd actix-app

# Edit .env file
nano .env
# Add your SENTRY_DSN and other variables
# Save and exit
```

### Step 2: Restart Actix Application

```bash
# Restart to apply new environment variables
docker compose restart

# OR stop and start
docker compose down
docker compose up -d

# Check logs to ensure it started successfully
docker compose logs -f
```

### Step 3: Verify Sentry Initialization

Look for these logs in Actix app:

```
Sentry/Glitchtip error tracking initialized
sentry_dsn=http://8d7f6c5e4d3c2b1a@localhost:8000/1234567890
environment=development
release=actix-openobserve@0.1.0
```

### Step 4: Trigger Test Error

```bash
# This endpoint simulates an error
curl http://localhost:8080/trigger-error
```

**Expected Response:**
```json
{
  "error": "database connection refused",
  "error_chain": [
    "attempting to fetch user data",
    "127.0.0.1:5432 unreachable"
  ],
  "request_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Step 5: Check Glitchtip Dashboard

1. Navigate to http://localhost:8000
2. Go to your project
3. Click on "Issues"
4. You should see a new issue with:
   - Error title: "database connection refused"
   - Stack trace
   - Request context
   - Tags (service.name, service.version, deployment.environment)
   - Breadcrumbs

### Step 6: Verify Error Details

**Issue should include:**

- ✅ Full error message
- ✅ Stack trace
- ✅ Error chain (context)
- ✅ Request ID
- ✅ HTTP method and endpoint
- ✅ Service tags
- ✅ Release version
- ✅ Environment name
- ✅ Event timestamp

### Step 7: Test Warning Endpoint

```bash
curl http://localhost:8080/trigger-warning
```

Warnings are logged but won't appear as errors in Glitchtip unless configured.

### Step 8: Verify Sample Rate

If you set `SENTRY_SAMPLE_RATE=0.5`, try triggering errors multiple times:

```bash
for i in {1..10}; do
  curl http://localhost:8080/trigger-error
done
```

You should see approximately 5 errors in Glitchtip (50% sample rate).

## Troubleshooting

### Issue 1: Errors Not Appearing in Glitchtip

**Symptoms:**
- Triggering errors works
- No errors in Glitchtip dashboard

**Solutions:**

1. **Check .env file:**
   ```bash
   # Verify SENTRY_DSN is set correctly
   docker exec -it actix_app env | grep SENTRY
   ```

2. **Check Actix logs:**
   ```bash
   docker logs actix_app | grep -i sentry
   ```

3. **Verify DSN format:**
   - Should be: `http://<key>@<host>/<project-id>`
   - No extra spaces or quotes

4. **Check Glitchtip accessibility:**
   ```bash
   # From Actix container
   docker exec -it actix_app curl http://localhost:8000/health/
   ```

5. **Increase sample rate:**
   ```bash
   # Set to 1.0 to capture all errors
   SENTRY_SAMPLE_RATE=1.0
   ```

### Issue 2: Invalid Sentry DSN Error

**Symptoms:**
```
Error: Invalid Sentry DSN: ...
```

**Solutions:**

1. **Verify DSN format:**
   ```bash
   # Correct format:
   http://abc123@localhost:8000/12345
   
   # Incorrect formats:
   https://localhost:8000 (missing key)
   abc123@localhost:8000 (missing protocol)
   http://abc123@localhost:8000 (missing project)
   ```

2. **Copy DSN directly from Glitchtip:**
   - Go to Project → Settings → Client Keys
   - Use "Copy" button instead of manual copying

3. **Check for hidden characters:**
   ```bash
   # Check for trailing spaces or quotes
   grep "SENTRY_DSN" .env | cat -A
   ```

### Issue 3: Connection Refused to Glitchtip

**Symptoms:**
```
Error: Failed to send event to Sentry: Connection refused
```

**Solutions:**

1. **Check Glitchtip is running:**
   ```bash
   docker ps | grep glitchtip_web
   ```

2. **Check Glitchtip health:**
   ```bash
   curl http://localhost:8000/health/
   ```

3. **Verify network connectivity:**
   ```bash
   # Both services should be on observability_openobserve_network
   docker network inspect observability_openobserve_network
   ```

4. **Check Glitchtip logs:**
   ```bash
   docker logs glitchtip_web | tail -50
   ```

### Issue 4: Environment Variables Not Loading

**Symptoms:**
- Actix app uses default values instead of .env values
- Sentry DSN shows default value in logs

**Solutions:**

1. **Verify .env file location:**
   ```bash
   # Should be in: actix-app/.env
   ls -la actix-app/.env
   ```

2. **Check file permissions:**
   ```bash
   # Should be readable by container
   chmod 644 actix-app/.env
   ```

3. **Restart Actix container:**
   ```bash
   cd actix-app
   docker compose down
   docker compose up -d
   ```

4. **Verify dotenvy is working:**
   ```bash
   # Check if dotenvy is loading .env
   docker logs actix_app | grep -i "env"
   ```

### Issue 5: Wrong Environment or Release

**Symptoms:**
- Errors show incorrect environment or release in Glitchtip

**Solutions:**

1. **Check .env values:**
   ```bash
   cat .env | grep SENTRY_ENVIRONMENT
   cat .env | grep SENTRY_RELEASE
   ```

2. **Verify no typos:**
   - Check for extra spaces
   - Verify no quote issues
   - Check variable names are exact

3. **Restart container:**
   ```bash
   docker compose restart
   ```

4. **Check Glitchtip tags:**
   - View issue details in Glitchtip
   - Check "Tags" section
   - Verify environment and release tags

### Issue 6: Sample Rate Not Working

**Symptoms:**
- All errors captured regardless of sample rate
- No errors captured even with sample rate = 1.0

**Solutions:**

1. **Verify variable is numeric:**
   ```bash
   # Correct:
   SENTRY_SAMPLE_RATE=1.0
   SENTRY_SAMPLE_RATE=0.5
   
   # Incorrect:
   SENTRY_SAMPLE_RATE=1
   SENTRY_SAMPLE_RATE="1.0"
   ```

2. **Check Rust code parsing:**
   - Review line 46 in main.rs
   - Ensure `.parse().ok()` is working

3. **Test with different values:**
   - Try 0.0 (no errors)
   - Try 1.0 (all errors)
   - Try 0.1 (10% of errors)

## Best Practices

### Security

1. **Never commit .env files:**
   ```bash
   # Add to .gitignore
   echo ".env" >> .gitignore
   
   # Create .env.example instead
   cp .env .env.example
   # Replace sensitive values with placeholders
   ```

2. **Use separate credentials per environment:**
   - Development DSN for local/staging
   - Production DSN for production
   - Never share production DSN

3. **Rotate DSNs regularly:**
   - Generate new DSN for major deployments
   - Remove old DSNs from Glitchtip

### Performance

1. **Use appropriate sample rates:**
   - Development: `1.0` (100%)
   - Staging: `0.5` (50%)
   - Production: `0.1` (10%) or lower for high-traffic

2. **Filter sensitive data:**
   - Set `send_default_pii: false`
   - Avoid logging passwords, tokens, or PII

3. **Monitor Glitchtip performance:**
   - Check queue processing
   - Monitor error ingestion rate
   - Review database performance

### Monitoring

1. **Set up alerts:**
   - Configure Glitchtip alerts for critical errors
   - Set thresholds for error rates
   - Enable email or Slack notifications

2. **Review error trends:**
   - Check daily/weekly error summaries
   - Identify recurring issues
   - Track error resolution progress

3. **Integrate with other tools:**
   - Link errors to Jira/GitHub issues
   - Use error data for incident response
   - Correlate with metrics in OpenObserve

### Release Tracking

1. **Always set SENTRY_RELEASE:**
   - Format: `<app-name>@<version>`
   - Example: `actix-openobserve@1.2.3`

2. **Tag releases in Git:**
   ```bash
   git tag -a v1.2.3 -m "Release 1.2.3"
   git push origin v1.2.3
   ```

3. **Track deployments:**
   - Associate Sentry releases with deployments
   - Monitor error rates per release
   - Identify problematic deployments quickly

## Advanced Configuration

### Custom Tags

Add custom tags to provide more context:

```rust
sentry::configure_scope(|scope| {
    // Add custom tags
    scope.set_tag("api_version", "v2");
    scope.set_tag("tenant_id", "tenant_123");
    scope.set_tag("request_type", "mutation");
});
```

### User Context

Track user information:

```rust
sentry::configure_scope(|scope| {
    scope.set_user(Some(sentry::User {
        email: Some("user@example.com".to_string()),
        username: Some("johndoe".to_string()),
        id: Some("user_12345".to_string()),
        ..Default::default()
    }));
});
```

### Breadcrumbs

Track events leading to errors:

```rust
sentry::add_breadcrumb(sentry::Breadcrumb {
    category: "http".into(),
    message: Some("GET /api/users".to_string()),
    level: sentry::Level::Info,
    ..Default::default()
});
```

### Custom Context

Add extra information:

```rust
sentry::configure_scope(|scope| {
    scope.set_extra("request_body", serde_json::json!({
        "user_id": "123",
        "action": "update_profile"
    }).into());
    
    scope.set_extra("database_query", "SELECT * FROM users WHERE id = 123".into());
});
```

### Performance Monitoring

Track request performance:

```rust
// In main.rs, add to ClientOptions
let client = sentry::init((
    dsn,
    ClientOptions {
        // ... other options
        traces_sample_rate: 0.1, // 10% sample rate for transactions
        ..Default::default()
    },
));
```

### Filtering Errors

Ignore certain errors:

```rust
// In ClientOptions
let client = sentry::init((
    dsn,
    ClientOptions {
        // ... other options
        before_send: Some(|event| {
            // Filter out specific errors
            if let Some(exception) = event.exception.values.first() {
                if exception.value.contains("expected error") {
                    return None; // Don't send this error
                }
            }
            Some(event)
        }),
        ..Default::default()
    },
));
```

### Integration with OpenTelemetry

Combine with OpenTelemetry tracing:

```rust
use sentry::integrations::tracing::EventFilter;

// In tracing setup
tracing_subscriber::registry()
    .with(sentry_tracing::layer().event_filter(EventFilter::Exception))
    // ... other layers
    .init();
```

## Quick Reference

### Essential Commands

```bash
# Start services
./start.sh

# Create Glitchtip admin
docker exec -it glitchtip_web python manage.py createsuperuser

# Update .env file
nano actix-app/.env

# Restart Actix app
cd actix-app && docker compose restart

# Test error tracking
curl http://localhost:8080/trigger-error

# Check Actix logs
docker logs -f actix_app

# Check Glitchtip status
docker ps | grep glitchtip
```

### Environment Variables Checklist

```bash
✅ SENTRY_DSN=http://<key>@<host>/<project-id>
✅ SENTRY_ENVIRONMENT=development|staging|production
✅ SENTRY_RELEASE=<app>@<version>
✅ SENTRY_SAMPLE_RATE=0.0-1.0
```

### Common DSN Formats

```
# Local development
http://abc123@localhost:8000/12345

# Staging
http://def456@staging.example.com/67890

# Production (HTTPS)
https://ghi789@glitchtip.example.com/13579
```

### Sample Rate Guidelines

| Environment | Sample Rate | Use Case |
|-------------|--------------|-----------|
| Development | `1.0` (100%) | Capture all errors for debugging |
| Staging | `0.5` (50%) | Balance detail vs volume |
| Production (Low Traffic) | `0.5` (50%) | Capture most errors |
| Production (High Traffic) | `0.1` (10%) | Reduce noise, focus on critical |
| Production (Critical Only) | `0.01` (1%) | Only severe errors |

## Conclusion

This guide provides a comprehensive approach to configuring Glitchtip DSN with your Rust Actix-web application. By following these steps, you'll have:

- ✅ Properly configured error tracking
- ✅ Structured environment management
- ✅ Clear understanding of the integration
- ✅ Testing and troubleshooting procedures
- ✅ Best practices for production use

For more information:

- **Glitchtip Documentation**: https://glitchtip.com/documentation
- **Sentry Rust SDK**: https://docs.sentry.io/platforms/rust/
- **Project README**: `/home/moo-tu/play_ground/observ_rust_stack/README.md`

---

**Last Updated**: 2024-03-12
**Version**: 1.0.0
**Status**: Production Ready