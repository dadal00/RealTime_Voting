FROM node:18 AS builder

WORKDIR /app

ARG VITE_SVELTE_PORT
ARG VITE_GO_URL
ARG VITE_WS_URL

ENV VITE_GO_URL=$VITE_GO_URL
ENV VITE_WS_URL=$VITE_WS_URL

COPY package*.json ./
RUN npm install
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE $VITE_SVELTE_PORT
CMD ["nginx", "-g", "daemon off;"]
