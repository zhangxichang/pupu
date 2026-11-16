-- Create "attachment" table
CREATE TABLE `attachment` (
  `hash` blob NOT NULL,
  `type` text NOT NULL,
  `data` blob NOT NULL,
  PRIMARY KEY (`hash`),
  CONSTRAINT `attachment_type` CHECK (type IN ('file', 'image', 'video', 'audio'))
);
-- Create "identity" table
CREATE TABLE `identity` (
  `id` text NOT NULL,
  `key` blob NOT NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `user_profile` FOREIGN KEY (`id`) REFERENCES `user_profile` (`id`)
);
-- Create "user_profile" table
CREATE TABLE `user_profile` (
  `id` text NOT NULL,
  `name` text NULL,
  `avatar_hash` blob NULL,
  `bio` text NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `profile_avatar` FOREIGN KEY (`avatar_hash`) REFERENCES `attachment` (`hash`)
);
-- Create "contact" table
CREATE TABLE `contact` (
  `identity_id` text NOT NULL,
  `profile_id` text NOT NULL,
  PRIMARY KEY (`identity_id`, `profile_id`),
  CONSTRAINT `ref_identity` FOREIGN KEY (`identity_id`) REFERENCES `identity` (`id`),
  CONSTRAINT `ref_profile` FOREIGN KEY (`profile_id`) REFERENCES `user_profile` (`id`)
);
-- Create "conversation" table
CREATE TABLE `conversation` (
  `id` text NOT NULL,
  `owner_identity` text NOT NULL,
  `type` text NOT NULL,
  `name` text NULL,
  `avatar_hash` blob NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `conversation_owner` FOREIGN KEY (`owner_identity`) REFERENCES `identity` (`id`),
  CONSTRAINT `conversation_avatar` FOREIGN KEY (`avatar_hash`) REFERENCES `attachment` (`hash`),
  CONSTRAINT `conversation_type` CHECK (type IN ('private', 'group'))
);
-- Create index "idx_conversation_owner" to table: "conversation"
CREATE INDEX `idx_conversation_owner` ON `conversation` (`owner_identity`);
-- Create "conversation_member" table
CREATE TABLE `conversation_member` (
  `conversation_id` text NOT NULL,
  `member_id` text NOT NULL,
  `last_read_message_id` integer NULL,
  PRIMARY KEY (`conversation_id`, `member_id`),
  CONSTRAINT `ref_conversation` FOREIGN KEY (`conversation_id`) REFERENCES `conversation` (`id`),
  CONSTRAINT `ref_member_profile` FOREIGN KEY (`member_id`) REFERENCES `user_profile` (`id`)
);
-- Create index "idx_conversation_member_profile" to table: "conversation_member"
CREATE INDEX `idx_conversation_member_profile` ON `conversation_member` (`member_id`);
-- Create "message" table
CREATE TABLE `message` (
  `id` integer NOT NULL PRIMARY KEY AUTOINCREMENT,
  `owner_conversation` text NOT NULL,
  `sender_id` text NOT NULL,
  `markdown` text NOT NULL,
  `timestamp` integer NOT NULL,
  CONSTRAINT `message_conversation` FOREIGN KEY (`owner_conversation`) REFERENCES `conversation` (`id`)
);
-- Create index "idx_owner_conversation" to table: "message"
CREATE INDEX `idx_owner_conversation` ON `message` (`owner_conversation`);
