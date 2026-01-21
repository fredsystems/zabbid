# Stage 1: Build the UI
FROM node:25.4.0-trixie as builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy source files
COPY . .

# Build the UI
RUN npm run build

# Stage 2: Serve with nginx
FROM nginx:1.29.4-trixie

# Remove default config
RUN rm /etc/nginx/conf.d/default.conf

# Copy SPA nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy built assets
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
