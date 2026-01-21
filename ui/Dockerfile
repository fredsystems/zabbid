# Stage 1: Build the UI
FROM node:20-bookworm as builder

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
FROM nginx:1.27-alpine

# Copy built assets from builder
COPY --from=builder /app/dist /usr/share/nginx/html

# Expose port
EXPOSE 80

# nginx will run as default user
CMD ["nginx", "-g", "daemon off;"]
