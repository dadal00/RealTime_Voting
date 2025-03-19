FROM golang:1.24 AS builder
WORKDIR /app
COPY go.mod go.sum ./
COPY *.go ./
RUN go mod download
RUN CGO_ENABLED=0 GOOS=linux go build -o /counter-go

ARG GO_PORT

FROM alpine:latest
COPY --from=builder /counter-go /app/
EXPOSE $GO_PORT
CMD ["/app/counter-go"]
