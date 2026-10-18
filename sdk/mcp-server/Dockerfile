# syntax=docker/dockerfile:1
#
# PayRaider MCP server (plugin) image.
# Build from the sdk/ directory so the TypeScript SDK is in the context:
#
#   docker build -f sdk/mcp-server/Dockerfile -t payraider-mcp sdk
#
FROM node:22-alpine AS build
WORKDIR /work

# The server depends on the SDK through a file: link, so build it first.
COPY typescript/package.json typescript/package-lock.json ./typescript/
RUN cd typescript && npm ci
COPY typescript ./typescript
RUN cd typescript && npm run build

COPY mcp-server/package.json mcp-server/package-lock.json ./mcp-server/
RUN cd mcp-server && npm ci
COPY mcp-server ./mcp-server
RUN cd mcp-server && npm run build:bundle

FROM node:22-alpine
ENV NODE_ENV=production \
    PAYRAIDER_MCP_TRANSPORT=http \
    HOST=0.0.0.0 \
    PORT=3333
WORKDIR /app
# One self-contained file: no node_modules in the runtime image.
COPY --from=build /work/mcp-server/bundle/server.mjs ./server.mjs
USER node
EXPOSE 3333
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -qO- "http://127.0.0.1:${PORT}/healthz" >/dev/null || exit 1
ENTRYPOINT ["node", "server.mjs"]
