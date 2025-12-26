/*
TODO:
Add Endpoints
  - GET  /api/events/codes                                                              - List available codes
  - POST /api/events/subscriptions?channel_id=XYZ&guild_id=ABC                          - List active subscriptions per Channel / Guild
  - POST /api/events/subscriptions/manage?subscribe=CODE&channel_id=XYZ&guild_id=ABC    - Subscribe
  - POST /api/events/subscriptions/manage?unsubscribe=CODE&channel_id=XYZ&guild_id=ABC  - Unsubscribe
*/
pub mod dispatcher;
pub mod models;
