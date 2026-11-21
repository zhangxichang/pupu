-- Create "new_attachment" table
CREATE TABLE `new_attachment` (
  `hash` blob NOT NULL,
  `type` text NOT NULL,
  `data` blob NOT NULL,
  `blake3` blob NULL,
  PRIMARY KEY (`hash`),
  CONSTRAINT `attachment_type` CHECK (type IN ('file', 'image', 'video', 'audio'))
);
-- Copy rows from old table "attachment" to new temporary table "new_attachment"
INSERT INTO `new_attachment` (`hash`, `type`, `data`) SELECT `hash`, `type`, `data` FROM `attachment`;
-- Drop "attachment" table after copying rows
DROP TABLE `attachment`;
-- Rename temporary table "new_attachment" to "attachment"
ALTER TABLE `new_attachment` RENAME TO `attachment`;
-- Create index "idx_attachment_blake3" to table: "attachment"
CREATE UNIQUE INDEX `idx_attachment_blake3` ON `attachment` (`blake3`);
-- Create "new_user_profile" table
CREATE TABLE `new_user_profile` (
  `id` text NOT NULL,
  `name` text NULL,
  `avatar_hash` blob NULL,
  `bio` text NULL,
  `endpoint_id` blob NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `profile_avatar` FOREIGN KEY (`avatar_hash`) REFERENCES `attachment` (`hash`)
);
-- Copy rows from old table "user_profile" to new temporary table "new_user_profile"
INSERT INTO `new_user_profile` (`id`, `name`, `avatar_hash`, `bio`) SELECT `id`, `name`, `avatar_hash`, `bio` FROM `user_profile`;
-- Drop "user_profile" table after copying rows
DROP TABLE `user_profile`;
-- Rename temporary table "new_user_profile" to "user_profile"
ALTER TABLE `new_user_profile` RENAME TO `user_profile`;
-- Create index "idx_user_profile_endpoint" to table: "user_profile"
CREATE UNIQUE INDEX `idx_user_profile_endpoint` ON `user_profile` (`endpoint_id`);
-- Create "new_contact" table
CREATE TABLE `new_contact` (
  `identity_id` text NOT NULL,
  `profile_id` text NOT NULL,
  `state` text NOT NULL DEFAULT 'accepted',
  PRIMARY KEY (`identity_id`, `profile_id`),
  CONSTRAINT `ref_identity` FOREIGN KEY (`identity_id`) REFERENCES `identity` (`id`),
  CONSTRAINT `ref_profile` FOREIGN KEY (`profile_id`) REFERENCES `user_profile` (`id`),
  CONSTRAINT `contact_state` CHECK (state IN ('pending_out', 'pending_in', 'accepted', 'rejected'))
);
-- Copy rows from old table "contact" to new temporary table "new_contact"
INSERT INTO `new_contact` (`identity_id`, `profile_id`) SELECT `identity_id`, `profile_id` FROM `contact`;
-- Drop "contact" table after copying rows
DROP TABLE `contact`;
-- Rename temporary table "new_contact" to "contact"
ALTER TABLE `new_contact` RENAME TO `contact`;
-- Create "new_conversation" table
CREATE TABLE `new_conversation` (
  `id` text NOT NULL,
  `owner_identity` text NOT NULL,
  `type` text NOT NULL,
  `name` text NULL,
  `avatar_hash` blob NULL,
  `network_id` text NULL,
  `topic_id` blob NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `conversation_owner` FOREIGN KEY (`owner_identity`) REFERENCES `identity` (`id`),
  CONSTRAINT `conversation_avatar` FOREIGN KEY (`avatar_hash`) REFERENCES `attachment` (`hash`),
  CONSTRAINT `conversation_type` CHECK (type IN ('private', 'group'))
);
-- Copy rows from old table "conversation" to new temporary table "new_conversation"
INSERT INTO `new_conversation` (`id`, `owner_identity`, `type`, `name`, `avatar_hash`) SELECT `id`, `owner_identity`, `type`, `name`, `avatar_hash` FROM `conversation`;
-- Drop "conversation" table after copying rows
DROP TABLE `conversation`;
-- Rename temporary table "new_conversation" to "conversation"
ALTER TABLE `new_conversation` RENAME TO `conversation`;
-- Create index "idx_conversation_owner" to table: "conversation"
CREATE INDEX `idx_conversation_owner` ON `conversation` (`owner_identity`);
-- Create index "idx_conversation_network" to table: "conversation"
CREATE UNIQUE INDEX `idx_conversation_network` ON `conversation` (`network_id`);
-- Create index "idx_conversation_topic" to table: "conversation"
CREATE UNIQUE INDEX `idx_conversation_topic` ON `conversation` (`topic_id`);
-- Add column "last_read_message_uid" to table: "conversation_member"
ALTER TABLE `conversation_member` ADD COLUMN `last_read_message_uid` blob NULL;
-- Create "new_message" table
CREATE TABLE `new_message` (
  `id` integer NOT NULL PRIMARY KEY AUTOINCREMENT,
  `owner_conversation` text NOT NULL,
  `sender_id` text NOT NULL,
  `markdown` text NOT NULL,
  `timestamp` integer NOT NULL,
  `uid` blob NOT NULL,
  `sender_seq` integer NOT NULL,
  `status` text NOT NULL DEFAULT 'sent',
  `signature` blob NULL,
  CONSTRAINT `message_conversation` FOREIGN KEY (`owner_conversation`) REFERENCES `conversation` (`id`),
  CONSTRAINT `message_status` CHECK (status IN ('pending', 'sent', 'failed'))
);
-- Copy rows from old table "message" to new temporary table "new_message"
-- Every existing message gets a fresh 16-byte uid and a per-sender sequence.
INSERT INTO `new_message` (`id`, `owner_conversation`, `sender_id`, `markdown`, `timestamp`, `uid`, `sender_seq`, `status`)
SELECT `id`, `owner_conversation`, `sender_id`, `markdown`, `timestamp`, randomblob(16),
       ROW_NUMBER() OVER (
           PARTITION BY `owner_conversation`, `sender_id`
           ORDER BY `timestamp`, `id`
       ),
       'sent'
FROM `message`;
-- Drop "message" table after copying rows
DROP TABLE `message`;
-- Rename temporary table "new_message" to "message"
ALTER TABLE `new_message` RENAME TO `message`;
-- Create index "idx_owner_conversation" to table: "message"
CREATE INDEX `idx_owner_conversation` ON `message` (`owner_conversation`);
-- Create index "idx_message_uid" to table: "message"
CREATE UNIQUE INDEX `idx_message_uid` ON `message` (`uid`);
-- Create index "idx_message_sender_seq" to table: "message"
CREATE UNIQUE INDEX `idx_message_sender_seq` ON `message` (`owner_conversation`, `sender_id`, `sender_seq`);
-- Create "outbox" table
CREATE TABLE `outbox` (
  `id` integer NOT NULL PRIMARY KEY AUTOINCREMENT,
  `conversation_id` text NULL,
  `target_endpoint_id` blob NULL,
  `message_uid` blob NULL,
  `kind` text NOT NULL,
  `payload` blob NOT NULL,
  `attempts` integer NOT NULL DEFAULT 0,
  `next_attempt_at` integer NOT NULL DEFAULT 0,
  `created_at` integer NOT NULL
);
-- Create index "idx_outbox_due" to table: "outbox"
CREATE INDEX `idx_outbox_due` ON `outbox` (`next_attempt_at`);
-- Create "conversation_sync" table
CREATE TABLE `conversation_sync` (
  `conversation_id` text NOT NULL,
  `peer_endpoint_id` blob NOT NULL,
  `sender_endpoint_id` blob NOT NULL,
  `last_seq` integer NOT NULL,
  PRIMARY KEY (`conversation_id`, `peer_endpoint_id`, `sender_endpoint_id`)
);
-- Create index "idx_conversation_sync_peer" to table: "conversation_sync"
CREATE INDEX `idx_conversation_sync_peer` ON `conversation_sync` (`peer_endpoint_id`);
