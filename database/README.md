# Data Model

## SET calls
`<IP>`

## HASH session:\<sessionName\>
```JSON
{
    code: String (SHA256),
    owner.id: String
}
```

## session:\<sessionName\>:\<ID\>
`access token (SHA256)`

## SET files:\<sessionName\>
`<filename>`

## HASH file:\<sessionName\>:\<filename\>
```JSON
{
    name: String,
    size: Number,
    owner.id: String (UID)
}
```

## accessAttempt:\<sessionName\>:\<IP\>
`<Nr of attempts>`