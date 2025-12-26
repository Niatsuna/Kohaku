/*
  TODO:
  Add Endpoints:
    - /api/keys/login                       - Login , Gain JWT
    - /api/keys/manage?owner=XYZ&revoke=ABC - Manage Keys, Revoke = Delete & Blacklist, Owner = Create anew
*/
pub mod api_key;
pub mod jwt;
pub mod models;
