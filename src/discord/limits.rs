#![allow(dead_code)]

const MARGIN: usize = 10;

pub const EMBED_TITLE_LENGTH: usize = 256 - MARGIN;
pub const EMBED_DESC_LENGTH: usize = 2048 - MARGIN;
pub const EMBED_FIELD_COUNT: usize = 25;
pub const EMBED_FIELD_NAME_LENGTH: usize = 256 - MARGIN;
pub const EMBED_FIELD_VALUE_LENGTH: usize = 1024 - MARGIN;
pub const EMBED_FOOTER_LENGTH: usize = 2048 - MARGIN;
pub const EMBED_AUTHOR_LENGTH: usize = 256 - MARGIN;
pub const TOTAL_EMBED_LENGTH: usize = 6000 - MARGIN;

pub const MESSAGE_LENGTH: usize = 2000 - MARGIN;
pub const NICK_LENGTH: usize = 32;
pub const ACTIVITY_LENGTH: usize = 128 - MARGIN;
pub const REPLY_LENGTH: usize = MESSAGE_LENGTH - NICK_LENGTH;
