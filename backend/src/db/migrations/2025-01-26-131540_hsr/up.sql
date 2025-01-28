CREATE TABLE hsr_items(
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL UNIQUE,
  rarity INTEGER NOT NULL CHECK (rarity >= 2 AND rarity <= 5),

  description VARCHAR,
  description_bg VARCHAR,

  types TEXT[] NOT NULL,
  sources TEXT[] NOT NULL,
  item_group INTEGER,

  api_url VARCHAR NOT NULL,
  wiki_url VARCHAR NOT NULL,
  img_url VARCHAR NOT NULL

);

CREATE INDEX idx_hsr_items_name on hsr_items(name);
CREATE INDEX idx_hsr_items_rarity on hsr_items(rarity);
CREATE INDEX idx_hsr_items_types on hsr_items(types);
CREATE INDEX idx_hsr_items_group on hsr_items(item_group);