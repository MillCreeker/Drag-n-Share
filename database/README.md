# Data Model

## HASH session:\<session.id\>

Session data.

### Data Structure

```JSON
{
    "name": String,
    "code": String (SHA256)
}
```

## session:\<session.name\>

Link of session name to session id.

### Data Structure

`<session.id>`

## SET files:\<session.id\>

List of files in a session.

### Data Structure

`<filename>`

## HASH files:\<session.id\>:\<filename\>

File metadata.

### Data Structure

```JSON
{
    "name": String,
    "size": Number,
    "owner.id": String (from JWT)
}
```

## created.sessions:\<ip\>
Keeps track of created sessions.
Users should only be able to create one session per IP.

### Data Structure
`<session.id>`

## access.attempts:\<session.id\>:\<ip\>

Count calls of an IP to join a session.
Too many calls will result in a rate limit.

### Data Structure

`<Nr of attempts>`

## SET calls

Will expire after 1 second.
To limit the call rate.

### Data Structure

`<ip>`
