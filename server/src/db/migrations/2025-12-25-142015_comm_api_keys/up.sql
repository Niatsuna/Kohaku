CREATE TABLE api_keys (
  id SERIAL PRIMARY KEY,
  hashed_key VARCHAR(255) NOT NULL,
  key_prefix VARCHAR(6) NOT NULL,
  owner VARCHAR(255) NOT NULL,
  scopes TEXT[] NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  UNIQUE (key_prefix, hashed_key)
);

CREATE INDEX idx_api_key_prefix ON api_keys(key_prefix);