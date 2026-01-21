# Phase 29H — Docker Compose Deployment

## Purpose

Provide a production-ready Docker Compose configuration for deploying the Zabbid system with all required services.

This sub-phase creates deployment infrastructure only. It does not modify application code or add new features.

---

## Scope

### 1. Docker Compose File

Create `docker-compose.yml` in project root with the following services:

#### MariaDB Service

- Image: `mariadb:latest` (or pinned version)
- Environment variables:
  - `MYSQL_ROOT_PASSWORD`
  - `MYSQL_DATABASE=zabbid`
  - `MYSQL_USER=zabbid`
  - `MYSQL_PASSWORD`
- Volume mounts:
  - `./data/mariadb:/var/lib/mysql` (persistent storage)
  - `./crates/persistence/migrations_mysql:/docker-entrypoint-initdb.d/migrations` (schema initialization)
- Health check configured
- Internal network only

#### Backend Service

- Build context: `./crates/server` (or root with appropriate Dockerfile)
- Dockerfile: Create multi-stage Rust build
- Environment variables:
  - `DATABASE_URL=mysql://zabbid:${MYSQL_PASSWORD}@mariadb:3306/zabbid`
  - `RUST_LOG=info`
  - `JWT_SECRET`
  - Additional configuration as needed
- Depends on: `mariadb`
- Exposes port internally (not published directly)
- Health check configured
- Wait for MariaDB readiness

#### UI Service

- Build context: `./ui` (or appropriate frontend directory)
- Dockerfile: Node.js build + serve
- Environment variables:
  - `VITE_API_URL=http://backend:8080` (or appropriate backend URL)
- Depends on: `backend`
- Exposes port internally (not published directly)

#### NGINX Service

- Image: `nginx:alpine`
- Configuration:
  - Reverse proxy to backend API
  - Serve UI static files (or proxy to UI service)
  - No SSL/TLS (HTTP only per phase scope)
- Volume mounts:
  - `./nginx.conf:/etc/nginx/nginx.conf:ro`
- Published port: `80:80`
- Depends on: `backend`, `ui`

### 2. Dockerfiles

#### Backend Dockerfile

Create `Dockerfile` in appropriate location:

```dockerfile
# Multi-stage build for Rust backend
FROM rust:1.83 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin zab-bid-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zab-bid-server /usr/local/bin/
EXPOSE 8080
CMD ["zab-bid-server"]
```

**Note:** Adjust paths, binary names, and dependencies as appropriate.

#### UI Dockerfile

Create `Dockerfile` in UI directory:

```dockerfile
# Multi-stage build for UI
FROM node:20 as builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
```

**Note:** Adjust paths and build commands as appropriate.

### 3. NGINX Configuration

Create `nginx.conf`:

```nginx
events {
    worker_connections 1024;
}

http {
    upstream backend {
        server backend:8080;
    }

    server {
        listen 80;
        server_name _;

        # API proxy
        location /api/ {
            proxy_pass http://backend;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # UI static files
        location / {
            root /usr/share/nginx/html;
            try_files $uri $uri/ /index.html;
        }
    }
}
```

**Note:** Adjust as needed based on actual UI/backend structure.

### 4. Environment Configuration

Create `.env.example` file:

```env
# MariaDB
MYSQL_ROOT_PASSWORD=changeme
MYSQL_PASSWORD=changeme

# Backend
JWT_SECRET=changeme_generate_secure_token
RUST_LOG=info

# UI
VITE_API_URL=http://localhost/api
```

**Security Note:** Actual `.env` file must not be committed. Add to `.gitignore`.

### 5. Docker Ignore Files

Create `.dockerignore` files to exclude unnecessary files from build context:

- `target/`
- `node_modules/`
- `.git/`
- `data/`
- Development/test files

### 6. Health Checks

Implement health checks for all services:

- **MariaDB:** `mysqladmin ping`
- **Backend:** HTTP GET to `/health` or similar endpoint
- **UI:** HTTP GET to root
- **NGINX:** HTTP GET to root

### 7. Volume Management

Define Docker volumes for persistent data:

- `mariadb_data` — MariaDB database files
- Consider volumes for logs if needed

### 8. Networking

Define internal network:

- `zabbid_network` — bridge network for inter-service communication
- Only NGINX port 80 is exposed to host

### 9. Startup Order

Ensure proper startup sequence:

1. MariaDB starts first
2. Backend waits for MariaDB health check
3. UI starts (may depend on backend)
4. NGINX starts last, depends on all services

Use `depends_on` with health check conditions.

### 10. Documentation

Create `docs/DEPLOYMENT.md` with:

- Prerequisites (Docker, Docker Compose)
- Quick start instructions
- Environment variable configuration
- Database migration steps
- Troubleshooting guide
- Security considerations

---

## Explicit Non-Goals

- No SSL/TLS configuration (HTTP only)
- No production secrets management (beyond .env)
- No orchestration (Kubernetes, Swarm)
- No CI/CD integration
- No monitoring/observability stack
- No backup/restore automation
- No high availability configuration
- No horizontal scaling

---

## Completion Checklist

- [ ] `docker-compose.yml` created
- [ ] Backend Dockerfile created
- [ ] UI Dockerfile created
- [ ] `nginx.conf` created
- [ ] `.env.example` created
- [ ] `.dockerignore` files created
- [ ] Health checks implemented for all services
- [ ] Startup dependencies configured
- [ ] Volume persistence configured
- [ ] Internal networking configured
- [ ] `docs/DEPLOYMENT.md` created
- [ ] Test deployment on clean system
- [ ] Verify MariaDB schema initialization
- [ ] Verify backend API accessibility
- [ ] Verify UI accessibility
- [ ] Verify inter-service communication
- [ ] Document troubleshooting steps
- [ ] `cargo xtask ci` passes (if applicable)
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Backend binary name or build configuration is unclear
- UI build process or output directory is unknown
- Database initialization process conflicts with existing migrations
- Health check endpoints don't exist in backend
- Environment variable requirements are uncertain
- Port conflicts with existing infrastructure
- Docker build context structure is ambiguous

---

## Risk Notes

- First-time deployment may require manual schema initialization
- Secrets in `.env` file are not secure for production
- No SSL means credentials transmitted in plaintext
- Volume permissions may cause issues on some systems
- Image sizes may be large without optimization
- Build times may be slow without layer caching
- Database migrations may need manual intervention on first run
- Existing development databases may conflict with Docker MariaDB
