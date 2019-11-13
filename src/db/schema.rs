table! {
    aliases (id) {
        id -> Integer,
        user_id -> Text,
        username -> Text,
        discriminator -> Text,
    }
}

table! {
    users (id) {
        id -> Text,
        first_seen -> Text,
        last_seen -> Text,
    }
}

joinable!(aliases -> users (user_id));

allow_tables_to_appear_in_same_query!(aliases, users,);
