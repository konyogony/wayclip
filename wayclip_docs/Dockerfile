FROM oven/bun:latest AS builder

WORKDIR /app

COPY package.json bun.lockb tsconfig.json next.config.mjs source.config.ts ./

RUN bun install --immutable

COPY . .

RUN bun run build

FROM node:20-slim

WORKDIR /app

COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

COPY --from=builder /app/package.json ./
COPY --from=builder /app/node_modules ./node_modules

EXPOSE 3003
ENV NODE_ENV=production
ENV PORT=3003
ENV HOST=0.0.0.0

CMD ["node", "server.js"]

