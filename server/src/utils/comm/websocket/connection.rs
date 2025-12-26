/*
- Connection Logic
  - Allow multiple Clients, All need to authenticate via JWT
  - If JWT expires: Close connection
  - As we only use the websocket for Server -> Client conversations: Ignore everything that is not a ping / pong that is incoming
*/
