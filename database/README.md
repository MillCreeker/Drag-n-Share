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

## file.req.ackn:\<session.id\>:\<filename\>:\<user.id\>

Session ID of user acknowledging request.

### Data Structure

`<request.id>`

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

## file.req.ready:\<request.id\>

Existance of this key indicates that the file is ready to be transmitted.

### Data Structure

`true`

## LIST file.req.chunks:\<request.id\>

Queue of file chunks + IV.
Left in, right out.

Chunks can only be of a certain size.
There is also a limit to the amount of chunks in the queue.

### Data Structure

`<chunk.nr>@<iv>@<chunk>`

## file.req.last.chunk:\<request.id\>

Last chunk number added to the queue.

### Data Structure

`<chunk.nr>`
