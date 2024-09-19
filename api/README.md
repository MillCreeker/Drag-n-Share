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
        "JWT": String
    }
}
```

## GET /session/:sessionName

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

## OPTIONS /session/:sessionId

Join a session.

### Headers

`Authorization: <access code encoded with SHA256>`

### Returns

```JSON
{
    "success": true,
    "response": {
        "JWT": String
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
            "isOwner": Boolean
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
        "isOwner": Boolean
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
