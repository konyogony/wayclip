FROM oven/bun:latest

WORKDIR /app

COPY . .

RUN bun install

RUN bun run build

EXPOSE 3030

CMD ["bun", "run", "start", "--port", "3030", "--hostname", "0.0.0.0"]
