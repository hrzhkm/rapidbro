FROM node:24-alpine AS builder

WORKDIR /app/fe

RUN corepack enable

COPY fe/package.json fe/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY fe/ ./

ARG VITE_BE_URL=http://localhost:3030
ENV VITE_BE_URL=${VITE_BE_URL}

RUN pnpm build

FROM node:24-alpine AS runtime

WORKDIR /app/fe

# Built server output
COPY --from=builder /app/fe/.output ./.output

# Prisma CLI and engines needed for migrate deploy
COPY --from=builder /app/fe/node_modules/.bin/prisma ./node_modules/.bin/prisma
COPY --from=builder /app/fe/node_modules/prisma ./node_modules/prisma
COPY --from=builder /app/fe/node_modules/@prisma ./node_modules/@prisma

# Migration files and config
COPY --from=builder /app/fe/prisma ./prisma
COPY --from=builder /app/fe/prisma.config.ts ./prisma.config.ts
COPY --from=builder /app/fe/package.json ./package.json

COPY fe/entrypoint.sh ./entrypoint.sh
RUN chmod +x ./entrypoint.sh

ENV NITRO_HOST=0.0.0.0
ENV NITRO_PORT=3000

EXPOSE 3000

CMD ["./entrypoint.sh"]
