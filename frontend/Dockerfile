# syntax=docker/dockerfile:1
# Multi-stage build: deps → builder → development / production

# ====================================================================
# Deps stage – install production dependencies only
# ====================================================================
FROM node:22-alpine AS deps

WORKDIR /app

# Copy dependency manifests
COPY package.json package-lock.json ./

# Install only production dependencies (for caching)
RUN npm ci --omit=dev

# ====================================================================
# Builder stage – full install + build
# ====================================================================
FROM node:22-alpine AS builder

WORKDIR /app

# Copy manifests and install all dependencies (including devDependencies)
COPY package.json package-lock.json ./
RUN npm ci

# Copy the rest of the source
COPY . .

# Build the production bundle
RUN npm run build

# ====================================================================
# Development stage – Vite dev server with hot-reload
# ====================================================================
FROM node:22-alpine AS development

WORKDIR /app

# Copy manifests and install all dependencies
COPY package.json package-lock.json ./
RUN npm ci

# Copy the source (so bind-mounts are optional)
COPY . .

EXPOSE 5173

# Run the Vite dev server on all interfaces
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]

# ====================================================================
# Production stage – nginx serving the SPA
# ====================================================================
FROM nginx:1.25-alpine AS production

# Create a non-root user for nginx worker processes
RUN addgroup -S vedo && \
    adduser -S -G vedo vedo && \
    sed -i 's/user  nginx;/user vedo;/' /etc/nginx/nginx.conf

# Remove the default nginx configuration
RUN rm /etc/nginx/conf.d/default.conf

# Copy custom nginx config with SPA routing
COPY docker/frontend/nginx.conf /etc/nginx/conf.d/default.conf

# Copy the production build output
COPY --from=builder /app/dist /usr/share/nginx/html

# Ensure the non-root user can read the files
RUN chown -R vedo:vedo /usr/share/nginx/html

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --spider -q http://localhost:80/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
