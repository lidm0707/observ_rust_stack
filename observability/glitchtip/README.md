# Glitchtip - Sentry-Compatible Error Tracking

## Overview

Glitchtip is an open-source error tracking platform that is 100% compatible with Sentry. It provides real-time error monitoring, stack trace collection, and performance monitoring for your applications.

## Architecture

This service consists of:

- **PostgreSQL**: Database for storing errors and events
- **Redis**: Cache and job queue for background processing
- **Glitchtip Web**: Main web application for viewing errors
- **Glitchtip Worker**: Background worker for processing events

## Quick Start

### Start the service

```bash
# Start all observability services (including Glitchtip)
./start.sh

# Or start only Glitchtip
cd observability/glitchtip
docker compose up -d
```

### Stop the service

```bash
# Stop all observability services (including Glitchtip)
./stop.sh

# Or stop only Glitchtip
cd observability/glitchtip
docker compose down
```

## Access Information

| Component | URL | Default Credentials |
|-----------|-----|---------------------|
| Web UI | http://localhost:8000 | Create admin account manually (see instructions below) |
| PostgreSQL | localhost:5432 | User: `glitchtip`, Password: `glitchtip_password`, DB: `glitchtip` |
| Redis | localhost:6379 | No authentication (default) |

## Network Configuration

Glitchtip uses the shared `observability_openobserve_network` to communicate with other services in the observability stack.

## Initial Setup

### 1. Create Admin Account

**After services are running, create the admin account interactively:**

```bash
# Run this command to create a superuser
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

### 2. Create an Organization

1. After login, you'll be prompted to create an organization
2. Enter an organization name (e.g., `my-company`)
3. Click "Create Organization"

### 3. Create a Project

1. Within your organization, click "Create Project"
2. Select platform: `Rust` (or `Other` if Rust isn't listed)
3. Enter project name (e.g., `actix-app`)
4. Click "Create Project"

### 4. Get DSN

1. After creating the project, you'll see the DSN (Data Source Name)
2. It looks like: `http://<public-key>@localhost:8000/<project-id>`
3. Copy this DSN to configure your application

## Configuration for Actix App

Update your `actix-app/.env` file with the following environment variables:

```bash
# Glitchtip/Sentry Configuration
SENTRY_DSN=http://<public-key>@localhost:8000/<project-id>
SENTRY_ENVIRONMENT=development
SENTRY_RELEASE=actix-openobserve@0.1.0
SENTRY_SAMPLE_RATE=1.0
```

### Environment Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `SENTRY_DSN` | The Sentry DSN from Glitchtip project | `http://8d7f6c5e4d3c2b1a@localhost:8000/1` | `http://abc123@localhost:8000/456` |
| `SENTRY_ENVIRONMENT` | Environment name (development, staging, production) | `development` | `staging` |
| `SENTRY_RELEASE` | Release identifier | `actix-openobserve@<version>` | `myapp@1.2.3` |
| `SENTRY_SAMPLE_RATE` | Error sampling rate (0.0 to 1.0) | `1.0` | `0.5` |

## Testing Error Tracking

### Manual Test

1. Ensure Glitchtip is running and configured
2. Start the Actix app with correct SENTRY_DSN
3. Trigger an error:
   ```bash
   curl http://localhost:8080/trigger-error
   ```
4. Check Glitchtip dashboard at http://localhost:8000
5. You should see the error with full stack trace

### Expected Error Data

Each error captured includes:
- Full stack trace
- Request information (URL, method, headers)
- Custom tags and extra context
- Breadcrumbs (trail of events leading to error)
- User information (if available)
- Release and environment information

## Monitoring and Logs

### View Logs

```bash
# View all Glitchtip logs
cd observability/glitchtip
docker compose logs -f

# View specific service logs
docker compose logs -f web
docker compose logs -f worker
docker compose logs -f postgres
docker compose logs -f redis
```

### Service Health

```bash
# Check service status
cd observability/glitchtip
docker compose ps

# Check individual container health
docker inspect glitchtip_web | grep -A 10 Health
```

## Features

- ✅ 100% Sentry-compatible
- ✅ Real-time error tracking
- ✅ Stack trace collection
- ✅ Source map support
- ✅ Release tracking
- ✅ User tracking
- ✅ Custom tags and context
- ✅ Breadcrumbs
- ✅ Alerts and notifications
- ✅ Performance monitoring (basic)
- ✅ Open source and self-hosted

## Troubleshooting

### Glitchtip won't start

1. Check if PostgreSQL is ready:
   ```bash
   docker compose logs postgres
   ```

2. Check if Redis is running:
   ```bash
   docker compose logs redis
   ```

3. Verify network exists:
   ```bash
   docker network inspect observability_openobserve_network
   ```

### Errors not appearing in dashboard

1. Verify SENTRY_DSN is correct in `.env`
2. Check Actix app logs for connection errors
3. Verify Glitchtip worker is processing events:
   ```bash
   docker compose logs worker
   ```
4. Check if sample rate is too low (try setting to `1.0`)

### Database connection errors

1. Check PostgreSQL health:
   ```bash
   docker compose ps postgres
   ```

2. Verify DATABASE_URL environment variable
3. Check PostgreSQL logs:
   ```bash
   docker compose logs postgres
   ```

### Performance issues

1. Increase worker count in `docker-compose.yml`:
   ```yaml
   environment:
     GUNICORN_WORKERS: "5"
   ```

2. Monitor Redis memory usage:
   ```bash
   docker stats glitchtip_redis
   ```

3. Check database performance:
   ```bash
   docker exec -it glitchtip_postgres psql -U glitchtip -d glitchtip -c "SELECT COUNT(*) FROM sentry_event;"
   ```

## Backup and Restore

### Backup Data

```bash
# Backup PostgreSQL database
docker exec glitchtip_postgres pg_dump -U glitchtip glitchtip > glitchtip_backup_$(date +%Y%m%d).sql

# Backup media files
docker run --rm -v glitchtip_glitchtip_data:/data -v $(pwd):/backup ubuntu tar czf /backup/glitchtip_media_$(date +%Y%m%d).tar.gz -C /data .
```

### Restore Data

```bash
# Restore PostgreSQL database
docker exec -i glitchtip_postgres psql -U glitchtip glitchtip < glitchtip_backup_20240101.sql

# Restore media files
docker run --rm -v glitchtip_glitchtip_data:/data -v $(pwd):/backup ubuntu tar xzf /backup/glitchtip_media_20240101.tar.gz -C /data
```

## Security Considerations

### For Development

- Create admin account manually using `docker exec -it glitchtip_web python manage.py createsuperuser`
- Services expose ports on localhost
- No SSL/TLS by default
- Open user registration is enabled by default (you can disable it in account settings)

### For Production

1. Change default superuser password by modifying `GLITCHTIP_SUPERUSER_PASSWORD` in `docker-compose.yml`
2. Change PostgreSQL password in `docker-compose.yml`
3. Enable SSL/TLS for the web service
4. Use strong SECRET_KEY
5. Set proper SENTRY_SAMPLE_RATE (e.g., 0.1 for 10% sampling)
6. Configure email for alerts
7. Restrict network access (don't expose PostgreSQL/Redis ports)
8. Enable authentication for Redis
9. Regular backups
10. Monitor resource usage

## Maintenance

### Regular Tasks

1. **Database vacuum**: Run monthly to reclaim space
   ```bash
   docker exec -it glitchtip_postgres psql -U glitchtip -d glitchtip -c "VACUUM ANALYZE;"
   ```

2. **Clean old events**: Use Glitchtip web UI to delete old events

3. **Update software**: Pull latest images
   ```bash
   docker compose pull
   docker compose up -d
   ```

4. **Monitor disk space**: Check volumes
   ```bash
   docker system df
   ```

## Integration with OpenObserve

While OpenObserve handles logs, metrics, and traces, Glitchtip is specialized for:

- **Error aggregation**: Grouping similar errors automatically
- **Stack trace analysis**: Better visualization of error chains
- **Issue tracking**: Assigning and managing error issues
- **Alerts**: Real-time notifications for new errors
- **User impact**: Understanding which users are affected by errors

They complement each other in a comprehensive observability stack.

## Additional Resources

- [Glitchtip Documentation](https://glitchtip.com/documentation)
- [Sentry Rust SDK Documentation](https://docs.sentry.io/platforms/rust/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)

## Support

For issues related to:
- **Glitchtip functionality**: Check [Glitchtip GitHub](https://github.com/glitchtip/glitchtip)
- **Rust SDK integration**: Check [Sentry Rust SDK](https://github.com/getsentry/sentry-rust)
- **This stack configuration**: Check project documentation