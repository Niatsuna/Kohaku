CREATE TABLE urls (
  id SERIAL PRIMARY KEY,
  addr VARCHAR NOT NULL UNIQUE,
  last_scraped TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_urls_addr ON urls(addr);