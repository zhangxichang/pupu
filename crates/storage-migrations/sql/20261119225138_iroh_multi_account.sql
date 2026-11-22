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
INSERT INTO `new_user_profile` (`id`, `name`, `avatar_hash`, `bio`, `endpoint_id`) SELECT `id`, `name`, `avatar_hash`, `bio`, `endpoint_id` FROM `user_profile`;
-- Drop "user_profile" table after copying rows
DROP TABLE `user_profile`;
-- Rename temporary table "new_user_profile" to "user_profile"
ALTER TABLE `new_user_profile` RENAME TO `user_profile`;
-- Create index "idx_user_profile_endpoint" to table: "user_profile"
CREATE INDEX `idx_user_profile_endpoint` ON `user_profile` (`endpoint_id`);
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
INSERT INTO `new_conversation` (`id`, `owner_identity`, `type`, `name`, `avatar_hash`, `network_id`, `topic_id`) SELECT `id`, `owner_identity`, `type`, `name`, `avatar_hash`, `network_id`, `topic_id` FROM `conversation`;
-- Drop "conversation" table after copying rows
DROP TABLE `conversation`;
-- Rename temporary table "new_conversation" to "conversation"
ALTER TABLE `new_conversation` RENAME TO `conversation`;
-- Create index "idx_conversation_owner" to table: "conversation"
CREATE INDEX `idx_conversation_owner` ON `conversation` (`owner_identity`);
-- Create index "idx_conversation_network" to table: "conversation"
CREATE INDEX `idx_conversation_network` ON `conversation` (`network_id`);
-- Create index "idx_conversation_owner_network" to table: "conversation"
CREATE UNIQUE INDEX `idx_conversation_owner_network` ON `conversation` (`owner_identity`, `network_id`);
-- Create index "idx_conversation_topic" to table: "conversation"
CREATE INDEX `idx_conversation_topic` ON `conversation` (`topic_id`);
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
INSERT INTO `new_message` (`id`, `owner_conversation`, `sender_id`, `markdown`, `timestamp`, `uid`, `sender_seq`, `status`, `signature`) SELECT `id`, `owner_conversation`, `sender_id`, `markdown`, `timestamp`, `uid`, `sender_seq`, `status`, `signature` FROM `message`;
-- Drop "message" table after copying rows
DROP TABLE `message`;
-- Rename temporary table "new_message" to "message"
ALTER TABLE `new_message` RENAME TO `message`;
-- Create index "idx_owner_conversation" to table: "message"
CREATE INDEX `idx_owner_conversation` ON `message` (`owner_conversation`);
-- Create index "idx_message_uid" to table: "message"
CREATE UNIQUE INDEX `idx_message_uid` ON `message` (`owner_conversation`, `uid`);
-- Create index "idx_message_sender_seq" to table: "message"
CREATE UNIQUE INDEX `idx_message_sender_seq` ON `message` (`owner_conversation`, `sender_id`, `sender_seq`);
