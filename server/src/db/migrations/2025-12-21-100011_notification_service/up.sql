CREATE TABLE notification_codes (
  code VARCHAR(255) PRIMARY KEY,
  last_used TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  description TEXT
);

CREATE TABLE notification_targets (
  id SERIAL PRIMARY KEY,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  code VARCHAR(255) NOT NULL,
  channel_id BIGINT NOT NULL,
  guild_id BIGINT NOT NULL,
  format TEXT,

  FOREIGN KEY (code) REFERENCES notification_codes(code),

  UNIQUE (code, channel_id),
  UNIQUE (code, guild_id)
);

CREATE INDEX idx_notification_targets_code ON notification_targets(code);
CREATE INDEX idx_notification_targets_guild ON notification_targets(guild_id);
CREATE INDEX idx_notification_targets_channel ON notification_targets(channel_id);
