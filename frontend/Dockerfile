FROM node:18 AS builder

WORKDIR /app

ARG PUBLIC_BACKEND_URL
ARG PUBLIC_WS_URL

ENV PUBLIC_BACKEND_URL=$PUBLIC_BACKEND_URL
ENV PUBLIC_WS_URL=$PUBLIC_WS_URL
ENV NODE_ENV=production

COPY package*.json ./
COPY svelte.config.js ./

RUN npm ci --omit=dev

COPY . .

RUN npm run build

FROM node:18-alpine

WORKDIR /app

COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules

ENV NODE_ENV=production
ENV PORT=5173

CMD PROTOCOL_HEADER=x-forwarded-proto HOST_HEADER=x-forwarded-host node build/index.js
