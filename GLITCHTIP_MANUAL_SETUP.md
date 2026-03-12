# Glitchtip Manual Superuser Implementation - Complete Summary

## Overview

This document summarizes the manual interactive approach for creating the initial admin account in Glitchtip (Sentry-compatible error tracking) for the observability stack.

## Implementation Decision

### Why Manual Interactive Approach?

After evaluating multiple approaches, we chose the **manual interactive superuser creation** method for the following reasons:

1. **Security**: No hardcoded credentials in configuration files
2. **Control**: Users choose their own email and password
3. **Standard Practice**: Follows Glitchtip's official documentation
4. **Flexibility**: Works across different deployment scenarios
5. **Simplicity**: No custom scripts or complex automation

### Alternatives Considered

| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| **Manual Interactive** ✅ | Secure, standard, flexible | Requires manual step | **Chosen** |
| Environment Variables | Fully automated | Hardcoded credentials, security risk | Rejected |
| Custom Shell Script | Automated, checks duplicates | Complexity, requires testing | Rejected |
| Django Management Command | Standard approach | Requires proper Django setup | Considered |

## Architecture Changes

### Files Modified

1. **`observability/glitchtip/docker-compose.yml`**
   - Removed: `SUPERUSER_EMAIL` and `SUPERUSER_PASSWORD` environment variables
   - Removed: `ENABLE_OPEN_USER_REGISTRATION` environment variable
   - Removed: Scripts volume mount
   - Simplified: Command to only run `migrate` and `runserver`

2. **`start.sh`**
   - Added: Instructions for manual superuser creation
   - Updated: Next Steps section with step-by-step guide

3. **`observability/glitchtip/README.md`**
   - Updated: Initial setup instructions
   - Updated: Security considerations
   - Removed: Auto-creation information

4. **`GLITCHTIP_INTEGRATION.md`**
   - Updated: Setup instructions
   - Updated: Security considerations
   - Updated: Next Steps

### Files Deleted

1. **`observability/glitchtip/scripts/`** - Entire directory removed
   - `create_superuser.py` - Python script approach
   - `create_superuser.sh` - Shell script approach

## Usage Guide

### Quick Start

```bash
# 1. Start all observability services
./start.sh

# 2. Wait for services to be healthy (watch the output)

# 3. Create Glitchtip admin account
docker exec -it glitchtip_web python manage.py createsuperuser

# 4. You will be prompted to enter:
#    - Email address (1 time)
#    - Password (2 times for confirmation)

# 5. Access Glitchtip Web UI
#    Navigate to: http://localhost:8000
#    Sign in with the credentials you just created
```

### Step-by-Step Instructions

#### Step 1: Start Services

```bash
./start.sh
```

Wait until you see:
```
[SUCCESS] All requested services have been started.
```

#### Step 2: Verify Services

```bash
# Check Glitchtip is running
docker ps | grep glitchtip_web

# Check Glitchtip logs (optional)
docker logs glitchtip_web
```

#### Step 3: Create Admin Account

```bash
docker exec -it glitchtip_web python manage.py createsuperuser
```

**Interactive Prompts:**

```
Email: admin@example.com
Password: **********
Password (again): **********
Superuser created successfully.
```

**What you'll see:**
- Email prompt (enter once)
- Password prompt (enter twice for confirmation)
- Confirmation message

#### Step 4: Sign In to Glitchtip

1. Navigate to: http://localhost:8000
2. Click "Sign in"
3. Enter email and password
4. You'll be logged in as superuser/admin

#### Step 5: Create Organization

1. After login, you may be prompted to create an organization
2. Or navigate to Settings → Organizations
3. Click "Create Organization"
4. Enter organization name (e.g., `my-company`)
5. Click "Create"

#### Step 6: Create Project

1. Navigate to your organization
2. Click "Create Project"
3. Enter project name (e.g., `actix-app`)
4. Select platform: `Rust` or `Other`
5. Click "Create"

#### Step 7: Get DSN

1. Navigate to your project settings
2. Find "Client Keys (DSN)" or "Integration Instructions"
3. Copy the DSN (looks like: `http://<public-key>@localhost:8000/<project-id>`)

#### Step 8: Configure Actix App

Update `actix-app/.env`:

```bash
# Glitchtip/Sentry Configuration
SENTRY_DSN=http://<public-key>@localhost:8000/<project-id>
SENTRY_ENVIRONMENT=development
SENTRY_RELEASE=actix-openobserve@0.1.0
SENTRY_SAMPLE_RATE=1.0
```

#### Step 9: Restart Actix App

```bash
cd actix-app
docker compose restart
```

#### Step 10: Test Error Tracking

```bash
curl http://localhost:8080/trigger-error
```

Check Glitchtip dashboard to see the captured error.

## Security Best Practices

### For Development

1. ✅ Use strong password when creating superuser
2. ✅ Keep services running on localhost only
3. ✅ Monitor Glitchtip logs regularly
4. ⚠️  Open user registration is enabled by default
5. ⚠️  No SSL/TLS by default

### For Production

1. ✅ **Strong Password**: Use a complex, unique password
2. ✅ **Disable Open Registration**: In Glitchtip settings, disable open signups
3. ✅ **Enable SSL/TLS**: Use reverse proxy (nginx, traefik) with HTTPS
4. ✅ **Strong SECRET_KEY**: Update in `docker-compose.yml`
5. ✅ **PostgreSQL Security**: Change default database password
6. ✅ **Redis Security**: Enable authentication
7. ✅ **Network Access**: Don't expose internal ports (5432, 6379)
8. ✅ **SENTRY_SAMPLE_RATE**: Set to 0.1 (10%) or lower in production
9. ✅ **Regular Backups**: Backup PostgreSQL database
10. ✅ **Monitor Resources**: Track CPU, memory, disk usage

### Password Guidelines

**Strong Password Requirements:**
- Minimum 12 characters
- Mix of uppercase and lowercase letters
- Include numbers and special characters
- Avoid common words or patterns
- Use a password manager
- Change regularly (every 90 days)

**Example Strong Password:**
```
K7#mP2$vL9!xR4&
```

## Troubleshooting

### Issue: Command Not Found

**Error:**
```
docker exec -it glitchtip_web python manage.py createsuperuser
command not found
```

**Solution:**
```bash
# Check if container is running
docker ps | grep glitchtip_web

# If not running, start services
cd observability/glitchtip
docker compose up -d
```

### Issue: Container Name Different

**Error:**
```
Error: No such container: glitchtip_web
```

**Solution:**
```bash
# List running containers
docker ps

# Find the actual Glitchtip web container name
# It might be something like: observability_glitchtip_web_1

# Use the actual name
docker exec -it <actual_container_name> python manage.py createsuperuser
```

### Issue: Database Not Ready

**Error:**
```
django.db.utils.OperationalError: connection refused
```

**Solution:**
```bash
# Wait for PostgreSQL to be healthy
docker compose ps
docker logs glitchtip_postgres

# Check if PostgreSQL is ready
docker exec glitchtip_postgres pg_isready -U glitchtip -d glitchtip

# Retry after PostgreSQL is healthy
docker exec -it glitchtip_web python manage.py createsuperuser
```

### Issue: Superuser Already Exists

**Error:**
```
Error: That email address is already in use.
```

**Solution:**
This is normal! The superuser has already been created. You can:

**Option 1: Sign In with Existing Account**
```
Just sign in to http://localhost:8000 with your existing credentials
```

**Option 2: Create Additional Superuser (if needed)**
```bash
docker exec -it glitchtip_web python manage.py createsuperuser
# Use a different email address
```

**Option 3: Reset Password (if forgotten)**
```bash
# This requires accessing Django admin or database
# Contact administrator for assistance
```

### Issue: Interactive Prompt Not Working

**Error:**
```
No interactive terminal available
```

**Solution:**
```bash
# Remove -it flags and use a different approach
# However, createsuperuser requires interactive input

# Alternative: Start container in interactive mode
docker compose run web python manage.py createsuperuser
```

### Issue: Permission Denied

**Error:**
```
Permission denied: /var/www/html/media
```

**Solution:**
```bash
# Check volume permissions
docker exec glitchtip_web ls -la /app/media

# Fix permissions if needed
docker exec glitchtip_web chown -R www-data:www-data /app/media
```

## Docker Compose Configuration

### Current Configuration

```yaml
version: "3.8"

services:
  web:
    image: glitchtip/glitchtip:latest
    container_name: glitchtip_web
    ports:
      - "8000:8000"
    volumes:
      - glitchtip_data:/app/media
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://glitchtip:glitchtip_password@postgres:5432/glitchtip
      DEFAULT_FROM_EMAIL: noreply@glitchtip.local
      EMAIL_URL: smtp://
      SECRET_KEY: super-secret-key-change-in-production
      GLITCHTIP_DOMAIN: http://localhost:8000
      ENABLE_EMAIL_REPLIES: "False"
      SENTRY_VERSION: "24.6.0"
      GUNICORN_WORKERS: "3"
    command: >
      sh -c "python manage.py migrate &&
             python manage.py runserver 0.0.0.0:8000"
```

### Key Points

- **No auto-creation**: Superuser must be created manually
- **Simple command**: Just `migrate` and `runserver`
- **Standard setup**: Follows Glitchtip best practices
- **Clean configuration**: No unnecessary environment variables

## Comparison: Manual vs Automated

### Manual Interactive Approach ✅

**Pros:**
- ✅ More secure (no hardcoded credentials)
- ✅ User chooses password
- ✅ Standard practice
- ✅ Simpler configuration
- ✅ Works across all deployment scenarios
- ✅ Easy to troubleshoot

**Cons:**
- ⚠️  Requires manual step after deployment
- ⚠️  Not fully automated
- ⚠️  Requires interactive terminal

### Automated Approach ❌

**Pros:**
- ✅ Fully automated
- ✅ No manual steps
- ✅ Faster deployment

**Cons:**
- ❌ Security risk (hardcoded credentials)
- ❌ More complex setup
- ❌ Requires custom scripts
- ❌ Harder to troubleshoot
- ❌ Deviates from standard practices

## Monitoring and Maintenance

### Regular Checks

**Daily:**
- Check Glitchtip is accessible: http://localhost:8000
- Monitor error rates in dashboard
- Review new user registrations

**Weekly:**
- Check container health: `docker compose ps`
- Review logs: `docker compose logs --tail=100`
- Verify database backups

**Monthly:**
- Update Glitchtip image: `docker compose pull`
- Review and rotate secrets
- Audit user accounts
- Check disk space usage

### Health Monitoring

```bash
# Check service status
docker compose ps

# Check Glitchtip health endpoint
curl -f http://localhost:8000/health/

# Check database connectivity
docker exec glitchtip_postgres pg_isready -U glitchtip -d glitchtip

# Check Redis connectivity
docker exec glitchtip_redis redis-cli ping
```

### Log Monitoring

```bash
# View all Glitchtip logs
docker compose logs -f web

# View specific logs
docker compose logs web | grep "superuser"
docker compose logs web | grep "error"

# View recent logs
docker compose logs --tail=50 web
```

## Integration with OpenObserve

### Complementary Roles

| Feature | OpenObserve | Glitchtip |
|---------|-------------|-----------|
| **Logs** | ✅ Excellent | ❌ No |
| **Metrics** | ✅ Excellent | ❌ Limited |
| **Traces** | ✅ Excellent | ⚠️ Basic |
| **Error Tracking** | ⚠️ Basic | ✅ Excellent |
| **Error Grouping** | ❌ No | ✅ Automatic |
| **Stack Traces** | ⚠️ Basic | ✅ Advanced |
| **Issue Management** | ❌ No | ✅ Full |
| **Alerts** | ✅ Yes | ✅ Yes |

### Data Flow

```
┌─────────────┐
│  Actix App  │
└──────┬──────┘
       │
       ├───────────────┬──────────────┐
       │               │              │
       ▼               ▼              ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│ OpenObserve│ │  Glitchtip  │ │  OpenObserve│
│   (Logs)   │ │  (Errors)   │ │ (Metrics)   │
└─────────────┘ └─────────────┘ └─────────────┘
```

Both platforms receive data from the Actix application simultaneously, providing complementary observability.

## Advanced Configuration

### Custom Django Settings

If you need additional Django settings, you can extend the configuration:

```yaml
environment:
  # Add custom Django settings
  DJANGO_SETTINGS_MODULE: glitchtip.settings
  DEBUG: "False"
  ALLOWED_HOSTS: "localhost,127.0.0.1"
  
  # Email configuration
  EMAIL_HOST: smtp.example.com
  EMAIL_PORT: "587"
  EMAIL_USE_TLS: "True"
  EMAIL_HOST_USER: noreply@example.com
  EMAIL_HOST_PASSWORD: your-email-password
```

### Performance Tuning

```yaml
environment:
  # Worker processes
  GUNICORN_WORKERS: "4"
  
  # Timeout settings
  GUNICORN_TIMEOUT: "120"
  
  # Database pool
  DB_MAX_CONNS: "20"
  
  # Redis settings
  CELERY_BROKER_URL: redis://redis:6379/0
  CELERY_RESULT_BACKEND: redis://redis:6379/0
  CELERY_TASK_ALWAYS_EAGER: "False"
```

### High Availability Setup

For production HA setup:

```yaml
services:
  web:
    deploy:
      replicas: 3
    # ... other config
  
  worker:
    deploy:
      replicas: 3
    # ... other config
  
  # Add load balancer
  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
    # ... load balancer config
```

## Backup and Recovery

### Database Backup

```bash
# Backup PostgreSQL
docker exec glitchtip_postgres pg_dump -U glitchtip glitchtip > glitchtip_backup_$(date +%Y%m%d).sql

# Backup with compression
docker exec glitchtip_postgres pg_dump -U glitchtip glitchtip | gzip > glitchtip_backup_$(date +%Y%m%d).sql.gz
```

### Database Restore

```bash
# Restore from backup
docker exec -i glitchtip_postgres psql -U glitchtip glitchtip < glitchtip_backup_20240312.sql

# Restore from compressed backup
gunzip -c glitchtip_backup_20240312.sql.gz | docker exec -i glitchtip_postgres psql -U glitchtip glitchtip
```

### Volume Backup

```bash
# Backup media volume
docker run --rm -v glitchtip_glitchtip_data:/data -v $(pwd):/backup ubuntu tar czf /backup/glitchtip_media_$(date +%Y%m%d).tar.gz -C /data .

# Restore media volume
docker run --rm -v glitchtip_glitchtip_data:/data -v $(pwd):/backup ubuntu tar xzf /backup/glitchtip_media_20240312.tar.gz -C /data
```

## Migration and Upgrades

### Upgrading Glitchtip

```bash
# Pull latest image
cd observability/glitchtip
docker compose pull web worker

# Restart services
docker compose up -d web worker

# Run migrations
docker compose exec web python manage.py migrate

# Collect static files
docker compose exec web python manage.py collectstatic --noinput
```

### Data Migration

When migrating between versions:

1. **Backup first**: Always backup before upgrading
2. **Review release notes**: Check for breaking changes
3. **Test in staging**: Never upgrade production without testing
4. **Monitor logs**: Watch for errors during migration
5. **Verify functionality**: Test after upgrade

## Support and Resources

### Official Resources

- **Glitchtip Documentation**: https://glitchtip.com/documentation
- **Glitchtip GitHub**: https://github.com/glitchtip/glitchtip
- **Sentry Rust SDK**: https://docs.sentry.io/platforms/rust/
- **Docker Compose Docs**: https://docs.docker.com/compose/

### Community Support

- **Glitchtip Discord**: https://discord.gg/glitchtip
- **GitHub Issues**: https://github.com/glitchtip/glitchtip/issues
- **Stack Overflow**: Tag questions with `glitchtip`

### Project-Specific Resources

- **Project README**: `/home/moo-tu/play_ground/observ_rust_stack/README.md`
- **Glitchtip README**: `observability/glitchtip/README.md`
- **Integration Summary**: `GLITCHTIP_INTEGRATION.md`

## Quick Reference

### Essential Commands

```bash
# Start services
./start.sh

# Stop services
./stop.sh

# Create superuser
docker exec -it glitchtip_web python manage.py createsuperuser

# View logs
docker compose logs -f web

# Check status
docker compose ps

# Restart services
docker compose restart

# Access shell
docker exec -it glitchtip_web sh
```

### Environment Variables

| Variable | Purpose | Default | Notes |
|----------|---------|---------|-------|
| `DATABASE_URL` | PostgreSQL connection | Required | Change password in production |
| `SECRET_KEY` | Django secret key | Change in production | Use strong random value |
| `GLITCHTIP_DOMAIN` | Glitchtip domain | `http://localhost:8000` | Update for production |
| `SENTRY_DSN` | Sentry DSN for Actix app | Required | Get from Glitchtip project |
| `SENTRY_ENVIRONMENT` | Environment name | `development` | Use `staging`/`production` |
| `SENTRY_RELEASE` | Release identifier | Auto-generated | Useful for release tracking |
| `SENTRY_SAMPLE_RATE` | Error sampling rate | `1.0` | Use `0.1` in production |

## Conclusion

The manual interactive superuser creation approach provides a secure, standard, and flexible way to initialize Glitchtip admin accounts. While it requires an additional manual step during initial setup, the benefits in terms of security and simplicity far outweigh the minor inconvenience.

This implementation follows Glitchtip's official documentation and best practices, ensuring a stable and maintainable error tracking solution for your observability stack.

---

**Last Updated**: 2024-03-12
**Version**: 1.0.0
**Status**: Production Ready