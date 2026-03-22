# Websockets & Events
To connect the rust backend and python client, Kohaku uses websockets for server-sided events (SSE). These websockets are only for receiving data from the server based on SSEs activity and are not for sending any data to the server.

## Websockets
The backend server allows multiple connections via a websocket manager.
Client must be authenticated and request access via a valid JWT access token.

### Limitations
- Authenticated clients only (via Access Token)
- One connection per API key (New connections will be discarded)
- Only receiving connections (Server -> Client)

At the current point of development, no limitations of how many clients can be connected are implemented. This feature was opted out for simplicity but will be implemented after a running prototype is available.

### Authentication flow
At first the user must generate an API key via the bootstrap key:
```bash
# 1) Login via bootstrap key
curl -X POST hhtp://SERVER_ADDR:SERVER_PORT/api/auth/login -H "X-API-Key: BOOTSTRAP_KEY"

# 2) Generate new api key
curl -X POST http://SERVER_ADDR:SERVER_PORT/api/auth/manage/create -H "Authorization: Bearer TOKEN" -H "Content-Type: application/json" -d '{"owner" : NAME, "scopes" : [SCOPE1, SCOPE2, ...]}'
```

Scopes are determined by `category:verb` and are the permissions this key has.
For the currently discussed websocket connection, this is `events:subscribe`.
> Note: Keys can only be generated and revoked by the bootstrap key.

After the generation the client must login via the newly generated key in the same way the bootstrap key logged in.
This will result in JWT tokens (Access and Refresh token).
> Note: Refresh token can be used to refresh the access token at `/api/auth/manage/refresh`

Use the access token during the connection attempt at `ws://SERVER_ADDR:SERVER_PORT/ws` to connect to the websocket.

## Events
SSEs can be triggered by any task and can be send via the event dispatcher.
Events are available through topics and subscriptions.

### Topics
Inspired by Apacha Kafkas topic design, topics are identifier to declare which data should be sent where.
```rust
pub struct Topic {
    /// Serial id in the database
    pub id: i32,
    /// Given name that must be used to subscribe and notify to this topic
    pub name: String,
    /// Description of the content of said topic
    pub description: String,
    //// Description what the conent will be formatted to (e.g. {content} = URL)
    pub details: Option<String>,
    //// Timestamp of creation in the database
    pub created_at: NaiveDateTime,
}
```

### Subscriptions
Subscriptions are components indicating that a specifc client wants to be informed if anything is sent to a topic.
Subscriptions are unique so that one API key can only subscribe with the same target data once but with different target datas multiple times.
This allows us for example to subscribe multiple discord channels to the same topic.
```rust
pub struct Subscription {
    /// Serial id in the database
    pub id: i32,
    /// Serial id of the corresponding topic
    pub topic_id: i32,
    /// Serial id of the corresponding API key
    pub key_id: i32,
    /// Optional target data to identify multiple subscriptions
    pub target_data: Option<Value>,
    /// TImestamp of creation in the database
    pub created_at: NaiveDateTime,
}
```

### Dispatcher
Under `server/src/utils/comm/events/dispatcher.rs` lies the corresponding `notify` function:
```rust
pub async fn notify(
    source: &str,
    topic: &str,
    instruction: &str,
    data: Value,
) -> Result<(), KohakuError> { ...
```
- `source` : An indicator for logging purposes which task triggered this event
- `topic` : The corresponding topic code
- `instruction` : If needed for the client different actions can be clarified here (e.g. CRUD operations like UPDATE and DELETE if necessary)
- `data` : The actual data that should be sent

The data of this event , the topic and subscription is then combined into `EventData` and `EventMessage`:

```rust
/// Actual inner data send in events
#[derive(Debug, Serialize, Deserialize)]
pub struct EventData {
    /// Actual content derived from the event (e.g. a message, link or anything else)
    pub content: serde_json::Value,
    /// Target data stored in the subscription for client sided handling (e.g. Discord channel and guild ids)
    pub target_data: Vec<serde_json::Value>,
}

/// Message struct that get send to the client fromt he dispatcher
#[derive(Debug, Serialize, Deserialize)]
pub struct EventMessage {
    /// Origin of the event on the servers side
    pub source: String,
    /// Topic name
    pub topic: String,
    /// Type of Event (e.g. Notify, Remove, etc.)
    pub instruction: String,
    /// Actual inner data send. Includes the content as well as the target data
    pub data: EventData,
}
```
which then get send to the connected client.