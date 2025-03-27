FROM node:20.12-alpine3.19 AS builder

ARG PUBLIC_BACKEND_URL
ARG PUBLIC_WS_URL
ARG PORT

ENV PUBLIC_BACKEND_URL=$PUBLIC_BACKEND_URL
ENV PUBLIC_WS_URL=$PUBLIC_WS_URL

WORKDIR /app

COPY package.json package-lock.json ./

RUN npm ci

COPY . .

RUN npm run build
RUN npm ci --prod

FROM node:20.12-alpine3.19

USER node:node

WORKDIR /app

COPY --from=builder --chown=node:node /app/build ./build
COPY --from=builder --chown=node:node /app/node_modules ./node_modules
COPY --chown=node:node package.json .

CMD [ "node", "build" ]
