# Base stage for shared configuration
FROM node:20-alpine AS base
WORKDIR /usr/src/app
COPY package*.json ./

# Builder stage: install all dependencies and build the app
FROM base AS builder
RUN npm ci
COPY . .
RUN npm run build

# Production stage: install only production dependencies and copy built assets
FROM base AS production
ARG NODE_ENV=production
ENV NODE_ENV=${NODE_ENV}
RUN npm ci --only=production
COPY --from=builder /usr/src/app/dist ./dist

# Expose the API port
EXPOSE 3000

# Start the application
CMD ["node", "dist/main"]
