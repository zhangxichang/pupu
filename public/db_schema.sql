BEGIN TRANSACTION;
CREATE TABLE IF NOT EXISTS "chat_records" (
	"id"	INTEGER,
	"timestamp"	TEXT NOT NULL DEFAULT (datetime('now')),
	"message"	TEXT NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT)
);
CREATE TABLE IF NOT EXISTS "friends" (
	"id"	TEXT,
	"name"	TEXT NOT NULL,
	"avatar"	BLOB,
	"bio"	TEXT,
	PRIMARY KEY("id")
);
CREATE TABLE IF NOT EXISTS "user_chat_record_index" (
	"id"	INTEGER,
	"user_id"	TEXT NOT NULL,
	"chat_id"	TEXT NOT NULL,
	"chat_record_id"	INTEGER NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT),
	UNIQUE("user_id","chat_record_id","chat_id"),
	FOREIGN KEY("chat_id") REFERENCES "friends"("id"),
	FOREIGN KEY("chat_record_id") REFERENCES "chat_records"("id"),
	FOREIGN KEY("user_id") REFERENCES "users"("id")
);
CREATE TABLE IF NOT EXISTS "user_friend_index" (
	"id"	INTEGER,
	"user_id"	TEXT NOT NULL,
	"friend_id"	TEXT NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT),
	UNIQUE("user_id","friend_id"),
	FOREIGN KEY("friend_id") REFERENCES "friends"("id"),
	FOREIGN KEY("user_id") REFERENCES "users"("id")
);
CREATE TABLE IF NOT EXISTS "users" (
	"id"	TEXT,
	"key"	BLOB NOT NULL UNIQUE,
	"name"	TEXT NOT NULL,
	"avatar"	BLOB,
	"bio"	TEXT,
	PRIMARY KEY("id")
);
COMMIT;
