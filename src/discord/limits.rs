#![allow(dead_code)]

pub const EMBED_TITLE_LENGTH: usize = 256;
pub const EMBED_DESC_LENGTH: usize = 2048;
pub const EMBED_FIELD_COUNT: usize = 25;
pub const EMBED_FIELD_NAME_LENGTH: usize = 256;
pub const EMBED_FIELD_VALUE_LENGTH: usize = 1024;
pub const EMBED_FOOTER_LENGTH: usize = 2048;
pub const EMBED_AUTHOR_LENGTH: usize = 256;
pub const TOTAL_EMBED_LENGTH: usize = 6000;

pub const MESSAGE_LENGTH: usize = 2000;
pub const NICK_LENGTH: usize = 32;
pub const ACTIVITY_LENGTH: usize = 128;
pub const REPLY_LENGTH: usize = MESSAGE_LENGTH - NICK_LENGTH - 5; // extra space for
