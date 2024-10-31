# Drag-n-Share

## Prerequisites

The services are intended to run on a Linux machine.
Tested on Ubuntu.

[Docker](https://docs.docker.com/engine/install/) must be installed.

The following ports must not be in use:

-   80
-   3000
-   6379
-   7878
-   7879

## Instructions

### Setup

Simply run `./setup.sh [dev|prod]` to launch the application in either development or production mode.

### Explanation

The bash script creates/overrides the necessary [redis.conf](./database/redis.conf) and [.env](./api/.env) files.
This simply defines the Redis password and JWT key.
Afterwards it launches all microservices using `sudo docker compose --profile [dev|prod] up`.

## Dev vs Prod

Dev containers use volumes to automatically reload upon file changes.
Prod containers aim at minimizing their size by a) not watching for file changes and b) compiling to a single script and executing that on a minimal container image.

[nginx](./nginx/) uses either localhost or drag-n-share.com for its reverse proxy depening on the mode.

## [API](./api/)

Consists of 2 microservices written in [Rust}(https://www.rust-lang.org/).

### [api.rs](./api/src/api.rs)

Handles the sessions.
Uses JWT to authenticate users.

### [transmittor.rs](./api/src/transmittor.rs)

Handles the file transmission process using a websocket.
Uses the same JWT from the API to authenticate users.

## [Web App](./web_app/)

A [Nuxt3](https://nuxt.com/) ([Vue](https://vuejs.org/)) app using [Tailwind CSS](https://tailwindcss.com/) for styling.

It has some utility scripts in the [utils folder](./web_app/public/utils/).

## File Transmission Process

This is arguably the most important part of the application.
It is a complicated piece of architecture due to its asynchronous nature.

It mainly involves these files:

-   [transmittor.rs](./api/src/transmittor.rs)
-   [transmittor.js](./web_app/public/utils/transmittor.js)
    -   used in [\[sessionId.vue\]](./web_app/pages/[sessionId].vue)

The interface/data structure parts can be found in their respective folders:

-   [API README](./api/README.md)
-   [DB README](./database/README.md)

### Process

#### Definitions

-   **Client S**: sender/provider of the file
-   **Client R**: receiver/requester of the file

---

-   A websocket connection is established for both clients, respectively
-   The clients call the `register` function in order to be registered on the server
    -   This starts a listening process
-   _Client S_ "uploads" a file.
    -   This registeres it in the session and makes it available for other clients
    -   It is also stored in the indexed database on the browser
-   _Client R_ requests a file using the `request-file` command
-   The server sends the `acknowledge-file-request` comamnd to _Client S_
-   _Client S_ sends the `acknowledge-file-request` command to the server
-   The server sends the `prepare-for-file-transfer` command to _Client R_
-   _Client R_ sends the `ready-for-file-transfer` command to the server
-   Then, the file transmission process starts until every file chunk has been transmitted:

-   The server sends the `send-next-chunk` command to _Client S_
-   _Client S_ sends the `add-chunk` command to the server
-   The server sends the `add-chunk` command to _Client R_
-   _Client R_ appends the chunk to its file string
-   _Client R_ sends the `received-chunk` command to the server
-   repeat ...

## Encryption

The transmitted chunks are encrypted using the AES-GCM 256 algorithm.
The keys are derived from the ECDH P-256 algorithm.
The public keys are sent to the other client during the `acknowledge-file-request` and `prepare-for-file-transfer`, respectively in order to derive the shared secret.
The secret is used to 1) encrypt and 2) decrypt the chunks.

The code for this is found in the [utils.js](./web_app/public/utils/utils.js) file.

This means that no chunk is neither readable on the server or on its way from/to the server. Only the clients can read the chunks.
Further security is added, by encrypting the chunks using an IV, which is randomly generated for each chunk and added to the request.
This ensures that the original keys can not be derived by the encrypted chunks themselves.
