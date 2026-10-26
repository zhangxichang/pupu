-- Create "attachment" table
CREATE TABLE `attachment` (
  `hash` blob NOT NULL,
  `mime` text NOT NULL,
  `data` blob NOT NULL,
  PRIMARY KEY (`hash`),
  CONSTRAINT `attachment_mime` CHECK (mime IN ('file', 'image', 'video', 'audio'))
);
-- Create "profile" table
CREATE TABLE `profile` (
  `id` text NOT NULL,
  `name` text NULL,
  `avatar_hash` blob NULL,
  `bio` text NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `profile_avatar` FOREIGN KEY (`avatar_hash`) REFERENCES `attachment` (`hash`)
);
-- Create "user" table
CREATE TABLE `user` (
  `id` text NOT NULL,
  `key` blob NOT NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `user_profile` FOREIGN KEY (`id`) REFERENCES `profile` (`id`)
);
-- Create "user_contact" table
CREATE TABLE `user_contact` (
  `user_id` text NOT NULL,
  `profile_id` text NOT NULL,
  PRIMARY KEY (`user_id`, `profile_id`),
  CONSTRAINT `ref_user` FOREIGN KEY (`user_id`) REFERENCES `user` (`id`),
  CONSTRAINT `ref_profile` FOREIGN KEY (`profile_id`) REFERENCES `profile` (`id`)
);
-- Create "conversation" table
CREATE TABLE `conversation` (
  `id` text NOT NULL,
  `owner_user` text NOT NULL,
  `type` text NOT NULL,
  `name` text NULL,
  `avatar_hash` blob NULL,
  PRIMARY KEY (`id`),
  CONSTRAINT `conversation_owner` FOREIGN KEY (`owner_user`) REFERENCES `user` (`id`),
  CONSTRAINT `conversation_avatar` FOREIGN KEY (`avatar_hash`) REFERENCES `attachment` (`hash`),
  CONSTRAINT `conversation_type` CHECK (type IN ('private', 'group'))
);
-- Create "conversation_member" table
CREATE TABLE `conversation_member` (
  `conversation_id` text NOT NULL,
  `member_id` text NOT NULL,
  PRIMARY KEY (`conversation_id`, `member_id`),
  CONSTRAINT `ref_conversation` FOREIGN KEY (`conversation_id`) REFERENCES `conversation` (`id`),
  CONSTRAINT `ref_member_profile` FOREIGN KEY (`member_id`) REFERENCES `profile` (`id`)
);
-- Create "message" table
CREATE TABLE `message` (
  `id` integer NOT NULL PRIMARY KEY AUTOINCREMENT,
  `owner_conversation` text NOT NULL,
  `sender_id` text NOT NULL,
  `content` text NOT NULL,
  `timestamp` integer NOT NULL,
  CONSTRAINT `message_conversation` FOREIGN KEY (`owner_conversation`) REFERENCES `conversation` (`id`)
);
-- Create index "idx_owner_conversation" to table: "message"
CREATE INDEX `idx_owner_conversation` ON `message` (`owner_conversation`);
