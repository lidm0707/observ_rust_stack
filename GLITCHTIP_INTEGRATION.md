# Glitchtip Integration Summary

This document provides a complete overview of the Glitchtip (Sentry-compatible error tracking) integration added to the observability stack.

## Overview

Glitchtip has been successfully integrated into the observability stack alongside OpenObserve. While OpenObserve handles logs, metrics, and traces, Glitchtip provides specialized error tracking with automatic error grouping, stack trace visualization, issue management, and real-time alerts.

## Changes Made

### 1. New Directory Structure

```
observ_rust_stack/observability/glitchtip/
├── docker-compose.yml
└── README.md
```

### 2. Glitchtip Docker Compose Service

**File**: `observability/glitchtip/docker-compose.yml`

A new Docker Compose service has been created with the following components:

- **PostgreSQL** (port 5432): Database for storing errors and events
- **Redis** (port 6379): Cache and job queue for background processing  
- **Glitchtip Web** (port 8000): Main web application
- **Glitchtip Worker**: Background worker for event processing

**Key Features**:
- All services use the shared `observability_openobserve_network`
- Health checks for all services
- Volume persistence for PostgreSQL and Glitchtip data
- Proper service dependencies

### 3. Rust Application Changes

#### Cargo.toml Updates

**File**: `observ_rust_stack/actix-app/Cargo.toml`

Added dependency:
```toml
sentry = "0.47.0"
```

#### main.rs Updates

**File**: `observ_rust_stack/actix-app/src/main.rs`

Added:

1. **Sentry Configuration Struct**:
   ```rust
   struct SentryConfig {
       dsn: String,
       environment: String,
       release: String,
       sample_rate: f32,
   }
   ```

2. **Environment Variable Support**:
   - `SENTRY_DSN`: Data Source Name for error reporting
   - `SENTRY_ENVIRONMENT`: Environment name (development/staging/production)
   - `SENTRY_RELEASE`: Release identifier
   - `SENTRY_SAMPLE_RATE`: Error sampling rate (0.0 to 1.0)

3. **Sentry Initialization Function**:
   - Parses and validates DSN
   - Configures Sentry client options
   - Sets up global scope with service metadata
   - Returns client guard for proper shutdown

4. **Error Capture Integration**:
   - Added error capture in `/trigger-error` endpoint
   - Captures errors with full context
   - Logs event ID for correlation

5. **Main Function Updates**:
   - Initializes Sentry before starting server
   - Logs Sentry configuration on startup
   - Proper shutdown handling

### 4. Shell Script Updates

#### start.sh Changes

**File**: `observ_rust_stack/start.sh`

Added:

- Glitchtip service startup
- Health checking for Glitchtip (90-second timeout)
- Glitchtip status display in service overview
- Access information for Glitchtip Web UI
- Log viewing commands for Glitchtip
- Management commands for Glitchtip
- Port mappings documentation
- Important notes about Sentry configuration

#### stop.sh Changes

**File**: `observ_rust_stack/stop.sh`

Added:

- Glitchtip service stopping
- Updated stop summary to include Glitchtip
- Enhanced data deletion warnings

### 5. Documentation

**File**: `observ_rust_stack/observability/glitchtip/README.md`

Created comprehensive documentation including:

- Architecture overview
- Quick start guide
- Access information and credentials
- Initial setup instructions (organization, project, DSN)
- Configuration examples for Actix app
- Testing procedures
- Monitoring and logging commands
- Troubleshooting guide
- Backup and restore procedures
- Security considerations
- Maintenance tasks
- Integration with OpenObserve

## Initial Setup Steps

### 1. Start All Services

```bash
./start.sh
```

This will start:
- OpenObserve (Logs, Metrics, Traces)
- Glitchtip (Error Tracking)
- Actix Web Application

### 2. Access Glitchtip Web UI

Navigate to: http://localhost:8000

### 3. Create Admin Account

**After services are running, create the admin account interactively:**

```bash
docker exec -it glitchtip_web python manage.py createsuperuser
```

**You will be prompted to enter:**
- Email address (1 time)
- Password (2 times for confirmation)

The first user created will be the superuser with admin privileges.

**Important notes:**
- This command only needs to be run once (for the first admin account)
- After creating the account, you can sign in at http://localhost:8000
- You can change your password later from the account settings

### 4. Create Organization

1. Sign in to http://localhost:8000 with the credentials you just created
2. Create an organization (e.g., `my-company`)

### 5. Create Project

1. Create a new project (e.g., `actix-app`)
2. Select platform: `Rust` or `Other`
3. Copy the DSN provided

### 4. Configure Actix Application

Update `actix-app/.env`:

```bash
# Glitchtip/Sentry Configuration
SENTRY_DSN=http://<public-key>@localhost:8000/<project-id>
SENTRY_ENVIRONMENT=development
SENTRY_RELEASE=actix-openobserve@0.1.0
SENTRY_SAMPLE_RATE=1.0
```

### 5. Restart Actix Application

```bash
cd actix-app
docker compose restart
```

### 6. Test Error Tracking

```bash
curl http://localhost:8080/trigger-error
```

Check Glitchtip dashboard at http://localhost:8000 to see the captured error.

## Service Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `SENTRY_DSN` | Glitchtip DSN from project settings | `http://8d7f6c5e4d3c2b1a@localhost:8000/1` | Yes |
| `SENTRY_ENVIRONMENT` | Environment name | `development` | No |
| `SENTRY_RELEASE` | Release identifier | `actix-openobserve@<version>` | No |
| `SENTRY_SAMPLE_RATE` | Error sampling rate | `1.0` | No |

### Docker Compose Services

| Service | Ports | Description |
|---------|-------|-------------|
| `glitchtip_postgres` | 5432 | PostgreSQL database |
| `glitchtip_redis` | 6379 | Redis cache and queue |
| `glitchtip_web` | 8000 | Glitchtip web application |
| `glitchtip_worker` | - | Background worker |

## Network Architecture

All services use the shared `observability_openobserve_network`:

```
┌─────────────────────────────────────────────────────────────┐
│           observability_openobserve_network                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  OpenObserve │  │   Glitchtip  │  │   Actix App  │     │
│  │              │  │              │  │              │     │
│  │  - Logs      │  │  - Errors    │  │  - Reports   │     │
│  │  - Metrics   │  │  - Issues    │  │  - Traces    │     │
│  │  - Traces    │  │  - Alerts    │  │  - Metrics   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Observability Stack Comparison

| Feature | OpenObserve | Glitchtip |
|---------|-------------|-----------|
| **Logs** | ✅ Excellent | ❌ No |
| **Metrics** | ✅ Excellent | ❌ Limited |
| **Traces** | ✅ Excellent | ⚠️ Basic |
| **Error Tracking** | ⚠️ Basic | ✅ Excellent |
| **Error Grouping** | ❌ No | ✅ Automatic |
| **Stack Trace Analysis** | ⚠️ Basic | ✅ Advanced |
| **Issue Management** | ❌ No | ✅ Full |
| **Alerts** | ✅ Yes | ✅ Yes |
| **User Impact** | ⚠️ Limited | ✅ Detailed |

The two platforms complement each other to provide comprehensive observability.

## Management Commands

### Start Services

```bash
# All services
./start.sh

# Only Glitchtip
cd observability/glitchtip
docker compose up -d
```

### Stop Services

```bash
# All services
./stop.sh

# Only Glitchtip
cd observability/glitchtip
docker compose down
```

### View Logs

```bash
# All Glitchtip logs
cd observability/glitchtip
docker compose logs -f

# Specific service
docker compose logs -f web
docker compose logs -f worker
```

### Check Status

```bash
# All services status
./start.sh  # Displays status at the end

# Glitchtip only
cd observability/glitchtip
docker compose ps
```

## Troubleshooting

### Common Issues

1. **Glitchtip won't start**: Check PostgreSQL and Redis are healthy
2. **Errors not appearing**: Verify SENTRY_DSN is correct and app is restarted
3. **Database connection errors**: Check PostgreSQL logs and connectivity
4. **Performance issues**: Increase worker count in docker-compose.yml

### Health Checks

```bash
# Check container health
docker inspect glitchtip_web | grep -A 10 Health

# Check network connectivity
docker network inspect observability_openobserve_network

# Test Glitchtip web interface
curl -f http://localhost:8000/health/
```

## Security Considerations
### For Development

- Create admin account manually using `docker exec -it glitchtip_web python manage.py createsuperuser`
- Services expose ports on localhost
- No SSL/TLS enabled by default
- Open user registration is enabled by default (can be disabled in account settings)

### Production Environment
1. Ensure you have created a strong admin account with secure password
2. Consider disabling open user registration in Glitchtip settings
3. Change PostgreSQL password in `docker-compose.yml`
4. Enable SSL/TLS
5. Use strong SECRET_KEY
6. Set appropriate SENTRY_SAMPLE_RATE (e.g., 0.1 for 10%)
7. Configure email alerts
8. Restrict network access
9. Enable Redis authentication
10. Regular backups
11. Monitor resource usage

## Backup and Restore

### Backup PostgreSQL

```bash
docker exec glitchtip_postgres pg_dump -U glitchtip glitchtip > backup.sql
```

### Restore PostgreSQL

```bash
docker exec -i glitchtip_postgres psql -U glitchtip glitchtip < backup.sql
```

### Backup Media Files

```bash
docker run --rm -v glitchtip_glitchtip_data:/data -v $(pwd):/backup ubuntu tar czf /backup/media.tar.gz -C /data .
```

## Maintenance Tasks

### Daily
- Monitor error rates in Glitchtip dashboard
- Check service health and logs

### Weekly
- Review and assign critical issues
- Check disk space usage

### Monthly
- Vacuum PostgreSQL database
- Clean up old events
- Review and update documentation
- Test backup and restore procedures

## Integration Points

### Actix App → Glitchtip
- Error reporting via Sentry SDK
- Automatic stack trace capture
- Request context and metadata
- Breadcrumbs for event trails

### Glitchtip → OpenObserve
- Complementary (not integrated)
- OpenObserve handles logs/metrics/traces
- Glitchtip handles error tracking and management

## Next Steps

1. ✅ Create admin account manually: `docker exec -it glitchtip_web python manage.py createsuperuser`
2. ✅ Complete initial Glitchtip setup (create organization and project)
3. ✅ Configure Actix app with SENTRY_DSN
4. ✅ Test error tracking with `/trigger-error` endpoint
5. ⏳ Configure email alerts for notifications
6. ⏳ Set up custom error tags and context
7. ⏳ Implement user tracking (if applicable)
8. ⏳ Configure release tracking for deployments
9. ⏳ Set up monitoring dashboards

## Resources

- [Glitchtip Documentation](https://glitchtip.com/documentation)
- [Sentry Rust SDK](https://docs.sentry.io/platforms/rust/)
- [OpenObserve Documentation](https://openobserve.ai/docs)
- [Project README](../README.md)

## Support

For issues related to:
- **Glitchtip functionality**: Check [Glitchtip GitHub](https://github.com/glitchtip/glitchtip)
- **Rust SDK integration**: Check [Sentry Rust SDK](https://github.com/getsentry/sentry-rust)
- **Stack configuration**: Check project documentation in README files

---

**Integration completed**: Glitchtip is now fully integrated into the observability stack alongside OpenObserve, providing comprehensive error tracking capabilities.