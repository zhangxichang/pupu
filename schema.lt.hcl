schema "main" {}

table "attachment" {
  schema = schema.main
  column "hash" {
    type = blob
  }
  column "mime" {
    type = text
  }
  column "data" {
    type = blob
  }
  primary_key {
    columns = [column.hash]
  }
  check "attachment_mime" {
    expr = "mime IN ('file', 'image', 'video', 'audio')"
  }
}

table "profile" {
  schema = schema.main
  column "id" {
    type = text
  }
  column "name" {
    type = text
    null = true
  }
  column "avatar_hash" {
    type = blob
    null = true
  }
  column "bio" {
    type = text
    null = true
  }
  primary_key {
    columns = [column.id]
  }
  foreign_key "profile_avatar" {
    columns     = [column.avatar_hash]
    ref_columns = [table.attachment.column.hash]
  }
}

table "user" {
  schema = schema.main
  column "id" {
    type = text
  }
  column "key" {
    type = blob
  }
  primary_key {
    columns = [column.id]
  }
  foreign_key "user_profile" {
    columns     = [column.id]
    ref_columns = [table.profile.column.id]
  }
}

table "user_contact" {
  schema = schema.main
  column "user_id" {
    type = text
  }
  column "profile_id" {
    type = text
  }
  primary_key {
    columns = [column.user_id, column.profile_id]
  }
  foreign_key "ref_user" {
    columns     = [column.user_id]
    ref_columns = [table.user.column.id]
  }
  foreign_key "ref_profile" {
    columns     = [column.profile_id]
    ref_columns = [table.profile.column.id]
  }
}

table "conversation" {
  schema = schema.main
  column "id" {
    type = text
  }
  column "owner_user" {
    type = text
  }
  column "type" {
    type = text
  }
  column "name" {
    type = text
    null = true
  }
  column "avatar_hash" {
    type = blob
    null = true
  }
  primary_key {
    columns = [column.id]
  }
  foreign_key "conversation_owner" {
    columns     = [column.owner_user]
    ref_columns = [table.user.column.id]
  }
  foreign_key "conversation_avatar" {
    columns     = [column.avatar_hash]
    ref_columns = [table.attachment.column.hash]
  }
  check "conversation_type" {
    expr = "type IN ('private', 'group')"
  }
}

table "conversation_member" {
  schema = schema.main
  column "conversation_id" {
    type = text
  }
  column "member_id" {
    type = text
  }
  primary_key {
    columns = [column.conversation_id, column.member_id]
  }
  foreign_key "ref_conversation" {
    columns     = [column.conversation_id]
    ref_columns = [table.conversation.column.id]
  }
  foreign_key "ref_member_profile" {
    columns     = [column.member_id]
    ref_columns = [table.profile.column.id]
  }
}

table "message" {
  schema = schema.main
  column "id" {
    type           = integer
    auto_increment = true
  }
  column "owner_conversation" {
    type = text
  }
  column "sender_id" {
    type = text
  }
  column "content" {
    type = text
  }
  column "timestamp" {
    type = integer
  }
  primary_key {
    columns = [column.id]
  }
  foreign_key "message_conversation" {
    columns     = [column.owner_conversation]
    ref_columns = [table.conversation.column.id]
  }
  index "idx_owner_conversation" {
    columns = [column.owner_conversation]
  }
}
