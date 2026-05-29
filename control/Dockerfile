# syntax=docker/dockerfile:1.7
FROM golang:1.22-alpine AS build
WORKDIR /src
COPY go.mod ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -ldflags="-s -w" -o /out/sentry-control ./cmd/sentry-control

FROM gcr.io/distroless/static-debian12
WORKDIR /app
COPY --from=build /out/sentry-control /app/sentry-control
COPY migrations /app/migrations
COPY config.example.yaml /app/config.yaml
EXPOSE 8081
ENTRYPOINT ["/app/sentry-control", "--config", "/app/config.yaml"]
