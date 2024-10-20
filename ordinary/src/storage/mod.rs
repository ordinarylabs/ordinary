mod graph;
pub use graph::*;

// public
// conversations.uuid.messages -> messages
// conversations.uuid.messages.uuid.replies -> messages
// users.uuid -> user
// users.uuid -> user

// private
// uuid.messages -> e2ee

// if we can't take in enough writes on the node, but have additional resources
// we should automatically shard the existing database into 2 databases.
