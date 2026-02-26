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

COPY --from=builder /app/fe/.output ./.output

ENV NITRO_HOST=0.0.0.0
ENV NITRO_PORT=3000

EXPOSE 3000

CMD ["node", ".output/server/index.mjs"]
