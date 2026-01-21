# Zabbid Deployment Guide

This document provides instructions for deploying Zabbid using Docker Compose.

---

## Prerequisites

Before deploying Zabbid, ensure you have the following installed:

- **Docker** (version 20.10 or later)
- **Docker Compose** (version 2.0 or later)
- **Git** (for cloning the repository)

### Verify Installation

```bash
docker --version
docker compose version
```

---

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/fredsystems/zabbid.git
cd zabbid
```

### 2. Configure Environment Variables

Copy the example environment file and update it with your values:

```bash
cp .env.example .env
```

Edit `.env` and update the following **required** variables:

```env
# Generate a strong root password
MYSQL_ROOT_PASSWORD=your_secure_root_password_here

# Generate a strong zabbid user password
MYSQL_PASSWORD=your_secure_zabbid_password_here

# Generate a secure JWT secret (minimum 32 characters)
# Example: openssl rand -base64 32
JWT_SECRET=your_secure_jwt_secret_at_least_32_characters_long
```

**Security Warning:** Never commit the `.env` file to version control. It is already included in `.gitignore`.

### 3. Generate Secure Secrets

Use these commands to generate secure values:

```bash
# Generate JWT secret (minimum 32 characters recommended)
openssl rand -base64 48

# Generate database passwords
openssl rand -base64 24
```

### 4. Start the Services

```bash
docker compose up -d
```

This command will:

1. Pull required Docker images
2. Build the backend and UI containers
3. Start MariaDB with automatic schema initialization
4. Start the backend API server
5. Start the UI server
6. Start NGINX as reverse proxy

### 5. Verify Deployment

Check that all services are running:

```bash
docker compose ps
```

All services should show status as "Up" and healthy.

### 6. Access the Application

Open your browser and navigate to:

```text
http://localhost
```

The UI should load and the API should be accessible at `/api/`.

---

## Service Architecture

The deployment consists of four services:

### MariaDB (`mariadb`)

- **Image:** mariadb:11.4
- **Purpose:** Primary database
- **Port:** 3306 (internal only)
- **Data:** Persisted in Docker volume `mariadb_data`
- **Health Check:** Built-in MariaDB health check

### Backend (`backend`)

- **Build:** Built from workspace root
- **Binary:** `zab-bid-server`
- **Port:** 8080 (internal only)
- **Dependencies:** Requires healthy MariaDB
- **Health Check:** HTTP GET to `/api/health`

### UI (`ui`)

- **Build:** Built from `ui/` directory
- **Framework:** React + Vite
- **Port:** 80 (internal only)
- **Dependencies:** Backend service

### NGINX (`nginx`)

- **Image:** nginx:1.27-alpine
- **Purpose:** Reverse proxy and static file server
- **Port:** 80 (published to host)
- **Routes:**
  - `/api/*` → Backend service
  - `/*` → UI static files

---

## Database Schema Initialization

On first startup, MariaDB will automatically run migrations from:

```text
crates/persistence/migrations_mysql/
```

These migrations create the complete Zabbid schema.

### Manual Migration

If you need to manually apply migrations after initial startup:

```bash
# Connect to the database container
docker compose exec mariadb mysql -u zabbid -p zabbid

# Or run migrations from host using Diesel CLI (if installed)
export DATABASE_URL="mysql://zabbid:YOUR_PASSWORD@localhost:3306/zabbid"
diesel migration run
```

---

## Service Management

### Start Services

```bash
docker compose up -d
```

### Stop Services

```bash
docker compose down
```

### Restart Services

```bash
docker compose restart
```

### Restart Individual Service

```bash
docker compose restart backend
```

### View Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f backend

# Last 100 lines
docker compose logs --tail=100 backend
```

### Rebuild After Code Changes

```bash
# Rebuild all services
docker compose build

# Rebuild specific service
docker compose build backend

# Rebuild and restart
docker compose up -d --build
```

---

## Data Persistence

### Database Data

MariaDB data is stored in a Docker volume named `mariadb_data`. This volume persists across container restarts and removals.

To backup the database:

```bash
# Create backup
docker compose exec mariadb mysqldump -u root -p zabbid > backup_$(date +%Y%m%d_%H%M%S).sql

# Restore from backup
docker compose exec -T mariadb mysql -u root -p zabbid < backup_20240101_120000.sql
```

### Volume Management

```bash
# List volumes
docker volume ls

# Inspect volume
docker volume inspect zabbid_mariadb_data

# Remove volume (WARNING: destroys all data)
docker compose down -v
```

---

## Networking

All services communicate over an internal Docker network named `zabbid_network`.

Only NGINX port 80 is exposed to the host.

### Access Services Internally

From the host, you can access services using:

```bash
# Backend API (via NGINX)
curl http://localhost/api/health

# Direct MariaDB connection (if port is published)
mysql -h 127.0.0.1 -u zabbid -p zabbid
```

---

## Troubleshooting

### Services Won't Start

1. Check Docker daemon is running:

   ```bash
   docker ps
   ```

2. Check logs for errors:

   ```bash
   docker compose logs
   ```

3. Verify environment variables:

   ```bash
   docker compose config
   ```

### MariaDB Health Check Failing

Wait 30-60 seconds for initial startup. MariaDB takes time to initialize.

Check logs:

```bash
docker compose logs mariadb
```

### Backend Can't Connect to Database

1. Verify MariaDB is healthy:

   ```bash
   docker compose ps mariadb
   ```

2. Check database credentials match in `.env`

3. Check backend logs:

   ```bash
   docker compose logs backend
   ```

### UI Not Loading

1. Check NGINX logs:

   ```bash
   docker compose logs nginx
   ```

2. Verify UI build completed successfully:

   ```bash
   docker compose logs ui
   ```

3. Check browser console for errors

### Port 80 Already in Use

If you have another service on port 80, edit `docker-compose.yml`:

```yaml
nginx:
  ports:
    - "8080:80" # Changed from 80:80
```

Then access at `http://localhost:8080`

### Permission Errors

Ensure the current user can access Docker:

```bash
# Add user to docker group (Linux)
sudo usermod -aG docker $USER

# Log out and back in for changes to take effect
```

### Database Connection Refused

Ensure services are on the same network:

```bash
docker network inspect zabbid_zabbid_network
```

All services should be listed as connected.

---

## Security Considerations

### Development vs Production

This Docker Compose configuration is designed for **local development and testing**.

For production deployment, consider:

1. **Use HTTPS/TLS:**
   - Add SSL certificates
   - Configure NGINX for HTTPS
   - Use Let's Encrypt or similar

2. **Secure Secrets Management:**
   - Use Docker secrets or external secret managers
   - Do not use `.env` files in production
   - Rotate secrets regularly

3. **Database Security:**
   - Use strong, unique passwords
   - Restrict database access
   - Enable audit logging
   - Regular backups

4. **Network Security:**
   - Use firewall rules
   - Restrict port access
   - Use private networks
   - Enable rate limiting

5. **Resource Limits:**
   - Set memory and CPU limits in `docker-compose.yml`
   - Monitor resource usage
   - Configure appropriate worker counts

6. **Monitoring:**
   - Add logging aggregation
   - Add metrics collection
   - Configure alerting
   - Health check monitoring

### HTTP Only Warning

**This configuration does NOT use HTTPS/TLS.**

Credentials and sensitive data are transmitted in plaintext over HTTP.

**DO NOT use this configuration on untrusted networks or for production without adding SSL/TLS.**

---

## Advanced Configuration

### Custom NGINX Configuration

Edit `nginx.conf` and rebuild:

```bash
docker compose restart nginx
```

### Backend Environment Variables

Additional backend configuration can be added to `docker-compose.yml`:

```yaml
backend:
  environment:
    DATABASE_URL: mysql://zabbid:${MYSQL_PASSWORD}@mariadb:3306/zabbid
    RUST_LOG: ${RUST_LOG:-info}
    JWT_SECRET: ${JWT_SECRET}
    BIND_ADDRESS: 0.0.0.0:8080
    # Add custom variables here
```

### Resource Limits

Add resource constraints to `docker-compose.yml`:

```yaml
backend:
  deploy:
    resources:
      limits:
        cpus: "2"
        memory: 2G
      reservations:
        cpus: "1"
        memory: 1G
```

---

## Updating Zabbid

### Update to Latest Version

```bash
# Pull latest code
git pull origin main

# Rebuild containers
docker compose build

# Restart with new images
docker compose up -d
```

### Database Migrations

If schema changes are required, migrations will run automatically on backend startup.

To manually check migration status:

```bash
docker compose exec backend diesel migration pending
```

---

## Uninstalling

### Stop and Remove Containers

```bash
docker compose down
```

### Remove All Data (WARNING)

```bash
# Remove containers, networks, and volumes
docker compose down -v

# Remove images
docker rmi zabbid-backend zabbid-ui
```

---

## Health Checks

All services include health checks:

- **MariaDB:** `healthcheck.sh --connect --innodb_initialized`
- **Backend:** `wget http://localhost:8080/api/health`
- **NGINX:** `wget http://localhost/`

Health status is visible in:

```bash
docker compose ps
```

---

## Support

For issues and questions:

- GitHub Issues: <https://github.com/fredsystems/zabbid/issues>
- Documentation: See project README.md and AGENTS.md

---

## License

Zabbid is released under the MIT License. See LICENSE file for details.
