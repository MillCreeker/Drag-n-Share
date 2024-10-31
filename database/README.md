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

---

---

---

## SET file.reqs:\<session.id\>

List of all file requests in a session.

### Data Structure

`<filename>`

## SET file.reqs:\<session.id\>:\<filename\>

List of users requesting a particular file.

### Data Structure

`<user.id>`

## file.req:\<session.id\>:\<filename\>:\<user.id\>

Public key of user requesting file.

### Data Structure

`<public.key>`

## SET file.req.users:\<request.id\>

List of users as part of a file request.
Will be validated using JWT.

### Data Structure

`<user.id>`

## SET file.reqs.sender:\<user.id\>

List of file requestes a user is a sender in.

### Data Structure

`<request.id>`

## SET file.reqs.receiver:\<user.id\>

List of file requestes a user is a receiver in.

### Data Structure

`<request.id>`

## HASH file.req.prep:\<request.id\>

Data for preparing file request.

### Data Structure

```JSON
{
    "filename": String,
    "public.key": String,
    "amount.of.chunks": Number
}
```

---

---

## chunk.curr:\<request.id\>

Current chunk that is being handled.

### Data Structure

`<chunk.nr>`

## chunk.req:\<request.id\>

Currently requested chunk from sender.

### Data Structure

`<chunk.nr>`

## chunk.sent:\<request.id\>

Currently sent chunk to receiver.

### Data Structure

`<chunk.nr>`

## chunk:\<request.id\>

Current chunk data.
Data includes (in order) _chunk.nr_, _IV_, _chunk_.

### Data Structure

`<chunk.nr>@<iv>@<chunk>`

## chunk.is.last:\<request.id\>

Indicates if the currently handled chunk is the last one.

### Data Structure

`<Boolean>`
