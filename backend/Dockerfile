FROM golang:1.24 AS builder
WORKDIR /app
COPY go.mod go.sum ./
COPY *.go ./
RUN go mod download
RUN CGO_ENABLED=0 GOOS=linux go build -o /counter-go

FROM alpine:latest
COPY --from=builder /counter-go /app/
CMD ["/app/counter-go"]
