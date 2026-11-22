schema "main" {}

table "attachment" {
  schema = schema.main
  column "hash" {
    type = blob
  }
  column "type" {
    type = text
  }
  column "data" {
    type = blob
  }
  column "blake3" {
    type = blob
    null = true
  }
  primary_key {
    columns = [column.hash]
  }
  index "idx_attachment_blake3" {
    unique  = true
    columns = [column.blake3]
  }
  check "attachment_type" {
    expr = "type IN ('file', 'image', 'video', 'audio')"
  }
}

table "identity" {
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
    ref_columns = [table.user_profile.column.id]
  }
}

table "user_profile" {
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
  column "endpoint_id" {
    type = blob
    null = true
  }
  primary_key {
    columns = [column.id]
  }
  foreign_key "profile_avatar" {
    columns     = [column.avatar_hash]
    ref_columns = [table.attachment.column.hash]
  }
  index "idx_user_profile_endpoint" {
    columns = [column.endpoint_id]
  }
}

table "contact" {
  schema = schema.main
  column "identity_id" {
    type = text
  }
  column "profile_id" {
    type = text
  }
  column "state" {
    type    = text
    default = "accepted"
  }
  primary_key {
    columns = [column.identity_id, column.profile_id]
  }
  foreign_key "ref_identity" {
    columns     = [column.identity_id]
    ref_columns = [table.identity.column.id]
  }
  foreign_key "ref_profile" {
    columns     = [column.profile_id]
    ref_columns = [table.user_profile.column.id]
  }
  check "contact_state" {
    expr = "state IN ('pending_out', 'pending_in', 'accepted', 'rejected')"
  }
}

table "conversation" {
  schema = schema.main
  column "id" {
    type = text
  }
  column "owner_identity" {
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
  column "network_id" {
    type = text
    null = true
  }
  column "topic_id" {
    type = blob
    null = true
  }
  primary_key {
    columns = [column.id]
  }
  foreign_key "conversation_owner" {
    columns     = [column.owner_identity]
    ref_columns = [table.identity.column.id]
  }
  foreign_key "conversation_avatar" {
    columns     = [column.avatar_hash]
    ref_columns = [table.attachment.column.hash]
  }
  check "conversation_type" {
    expr = "type IN ('private', 'group')"
  }
  index "idx_conversation_owner" {
    columns = [column.owner_identity]
  }
  index "idx_conversation_network" {
    columns = [column.network_id]
  }
  index "idx_conversation_owner_network" {
    unique  = true
    columns = [column.owner_identity, column.network_id]
  }
  index "idx_conversation_topic" {
    columns = [column.topic_id]
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
  column "last_read_message_id" {
    type = integer
    null = true
  }
  column "last_read_message_uid" {
    type = blob
    null = true
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
    ref_columns = [table.user_profile.column.id]
  }
  index "idx_conversation_member_profile" {
    columns = [column.member_id]
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
  column "markdown" {
    type = text
  }
  column "timestamp" {
    type = integer
  }
  column "uid" {
    type = blob
  }
  column "sender_seq" {
    type = integer
  }
  column "status" {
    type    = text
    default = "sent"
  }
  column "signature" {
    type = blob
    null = true
  }
  primary_key {
    columns = [column.id]
  }
  foreign_key "message_conversation" {
    columns     = [column.owner_conversation]
    ref_columns = [table.conversation.column.id]
  }
  check "message_status" {
    expr = "status IN ('pending', 'sent', 'failed')"
  }
  index "idx_owner_conversation" {
    columns = [column.owner_conversation]
  }
  index "idx_message_uid" {
    unique  = true
    columns = [column.owner_conversation, column.uid]
  }
  index "idx_message_sender_seq" {
    unique  = true
    columns = [column.owner_conversation, column.sender_id, column.sender_seq]
  }
}

table "outbox" {
  schema = schema.main
  column "id" {
    type           = integer
    auto_increment = true
  }
  column "conversation_id" {
    type = text
    null = true
  }
  column "target_endpoint_id" {
    type = blob
    null = true
  }
  column "message_uid" {
    type = blob
    null = true
  }
  column "kind" {
    type = text
  }
  column "payload" {
    type = blob
  }
  column "attempts" {
    type    = integer
    default = 0
  }
  column "next_attempt_at" {
    type    = integer
    default = 0
  }
  column "created_at" {
    type = integer
  }
  primary_key {
    columns = [column.id]
  }
  index "idx_outbox_due" {
    columns = [column.next_attempt_at]
  }
}

table "conversation_sync" {
  schema = schema.main
  column "conversation_id" {
    type = text
  }
  column "peer_endpoint_id" {
    type = blob
  }
  column "sender_endpoint_id" {
    type = blob
  }
  column "last_seq" {
    type = integer
  }
  primary_key {
    columns = [column.conversation_id, column.peer_endpoint_id, column.sender_endpoint_id]
  }
  index "idx_conversation_sync_peer" {
    columns = [column.peer_endpoint_id]
  }
}
