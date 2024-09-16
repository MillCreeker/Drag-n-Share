# Drag-n-Share

## Instructions

### Setup
`redis.conf` file in [database](./database/) folder:
```
requirepass "123456"
```

`.env` file in [api](./api/) folder:
```
DATABASE_PASSWORD=123456
JWT_KEY=abc123
```

`.env` file in [web_app](./web_app/) folder:
```
API_KEY=abc123
```

### Developer Mode
watches for changes

`sudo docker compose --profile dev up`

### Production Mode
Release version containers

`sudo docker compose --profile prod up`