# API Routes

## GET /

Ping for health check.

### Returns

```JSON
{
    "success": true,
    "response": timestamp
}
```

## GET /session

Retreives session if JWT indicates that the user already hosts a session.

### Headers

`Authorization: Bearer <JWT>`

### Returns

```JSON
{
    "success": true,
    "response": {
        "sessionName": String,
        "sessionId": String,
        "accessCode": String
    }
}
```

## POST /session

Creates a new session.

### Returns

```JSON
{
    "success": true,
    "response": {
        "sessionName": String,
        "sessionId": String,
        "accessCode": String,
        "jwt": String
    }
}
```

## GET /idForName/:session_name

Get sessionId for sessionName.

### Returns

```JSON
{
    "success": true,
    "response": {
        "sessionId": String
    }
}
```

## GET /session/:sessionId

Returns session metadata.

### Returns

```JSON
{
    "success": true,
    "response": {
        "sessionName": String
    }
}
```

## GET /access/:sessionId

Join a session.

### Headers

`Authorization: <access code encoded with SHA256>`

### Returns

```JSON
{
    "success": true,
    "response": {
        "jwt": String
    }
}
```

## PUT /session/:sessionId

Only possible, if the JWT indicates the user is the owner of the session.
Create a new accessCode.
Optionally update session name.

### Headers

`Authorization: Bearer <JWT>`

### Body

```JSON
{
    "name": String
}
```

### Returns

```JSON
{
    "success": true,
    "response": {
        "accessCode": String
    }
}
```

## DELETE /session/:sessionId

Only possible, if the JWT indicates the user is the owner of the session.
Permanently deletes a session.

### Headers

`Authorization: Bearer <JWT>`

### Returns

```JSON
{
    "success": true,
    "response": <confirmation message>
}
```

## GET /files/:sessionId

Returns all files in a session.

### Headers

`Authorization: Bearer <JWT>`

### Returns

```JSON
{
    "success": true,
    "response": [
        {
            "name": String,
            "size": Number,
            "is_owner": Boolean
        }
    ]
}
```

## POST /files/:sessionId

Add files to a session.

### Headers

`Authorization: Bearer <JWT>`

### Body

```JSON
[
    {
        "name": String,
        "size": Number
    }
]
```

### Returns

```JSON
{
    "success": true,
    "response": <confirmation message>
}
```

## GET /files/:sessionId/:filename

Get file metadata.

### Headers

`Authorization: Bearer <JWT>`

### Returns

```JSON
{
    "success": true,
    "response": {
        "name": String,
        "size": Number,
        "is_owner": Boolean
    }
}
```

## DELETE /files/:sessionId/:filename

Only possible, if the JWT indicates the user is the owner of the session or the file.

### Headers

`Authorization: Bearer <JWT>`

### Returns

```JSON
{
    "success": true,
    "response": <confirmation message>
}
```

# Transmittor

## GET /

Ping for health check.

### Returns

```JSON
{
    "success": true,
    "response": timestamp
}
```

## GET /session/:sessionId

Connects to a websocket.
Based on the `command`, a different function is being executed.

The `data` parameter is an object that depends on the `command`.

### Request

Requires a valid JWT of a user who joined the session.

#### Body

```JSON
{
    "jwt": String,
    "command": String,
    "data": Object,
}
```

#### Returns

```JSON
{
    "success": Boolean,
    "response": String,
}
```

### Commands - Request

#### register

```JSON
"data": {}
```

#### request-file

```JSON
"data": {
    "public_key": String,
    "filename": String
}
```

#### acknowledge-file-request

```JSON
"data": {
    "public_key": String,
    "amount_of_chunks": Number,
    "filename": String,
    "user_id": String,
}
```

#### ready-for-file-transfer

```JSON
"data": {
    "request_id": String,
}
```

#### add-chunk

```JSON
"data": {
    "request_id": String,
    "is_last_chunk": Boolean,
    "chunk_nr": Number,
    "chunk": String,
    "iv": String
}
```

### Messages

Messages send from the websocket indipendently.

### Sends

```JSON
{   
    "request_id": String,
    "command": String,
    "data": Object,
}
```

### Commands - Messages

#### acknowledge-file-request

```JSON
"data": {
    "public_key": String,
    "filename": String,
    "user_id": String
}
```

#### prepare-for-file-transfer

```JSON
"data": {
    "public_key": String,
    "filename": String,
    "amount_of_chunks": Number
}
```

#### send-next-chunk

```JSON
"data": {
    "last_chunk_nr": Number
}
```

#### add-chunk

```JSON
"data": {
    "is_last_chunk": Boolean,
    "chunk_nr": Number,
    "chunk": String,
    "iv": String
}
```
