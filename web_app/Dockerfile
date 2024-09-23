FROM oven/bun:1.1.29

WORKDIR /app

COPY package.json bun.lockb /app/

RUN bun install

COPY . /app/

RUN bun run build

EXPOSE 3000

CMD ["NUXT_PUBLIC_API_URI=https://drag-n-share.com", "bun", ".output/server/index.mjs"]