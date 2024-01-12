FROM node:20-slim AS base

# Setup PNPM
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable

WORKDIR /app/
COPY ./ ./

# Development 
FROM base as development
VOLUME [ "/app" ]
CMD ["pnpm", "run", "serve"]

# Install production dependencies only
FROM base AS dependencies-production
RUN --mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --prod --frozen-lockfile

# Create the build
FROM base AS build
RUN --mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --frozen-lockfile
RUN pnpm run build

# Production
FROM base as backend-production
COPY --from=dependencies-production /app/node_modules /app/node_modules
COPY --from=build /app/packages/backend/dist /app/dist
EXPOSE 3000
CMD ["pnpm", "run", "start"]


