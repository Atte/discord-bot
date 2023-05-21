use async_trait::async_trait;
use serenity::{
    constants::MESSAGE_CODE_LIMIT,
    model::prelude::*,
    prelude::*,
    utils::{content_safe, ContentSafeOptions},
    Result,
};

use crate::word_chunks::WordChunks;

#[async_trait]
pub trait SafeReply {
    async fn safe_reply(&self, ctx: &Context, content: impl Into<String> + Send)
        -> Result<Message>;
    async fn safe_reply_paged(
        &self,
        ctx: &Context,
        content: impl Into<String> + Send,
        max_pages: usize,
    ) -> Result<Vec<Message>>;
}

#[async_trait]
impl SafeReply for Message {
    #[inline]
    async fn safe_reply(
        &self,
        ctx: &Context,
        content: impl Into<String> + Send,
    ) -> Result<Message> {
        Ok(self.safe_reply_paged(ctx, content, 1).await?.remove(0))
    }

    async fn safe_reply_paged(
        &self,
        ctx: &Context,
        content: impl Into<String> + Send,
        max_pages: usize,
    ) -> Result<Vec<Message>> {
        let mut safe_opts = ContentSafeOptions::default().show_discriminator(false);
        if let Some(guild_id) = self.guild_id {
            safe_opts = safe_opts.display_as_member_from(guild_id);
        }

        let response = content_safe(ctx, content.into(), &safe_opts, &self.mentions);
        let response: Vec<_> = WordChunks::from_str(&response, MESSAGE_CODE_LIMIT).collect();

        let mut responses = Vec::with_capacity(response.len().min(max_pages));
        let mut reply_to = self.clone();
        for chunk in response.into_iter().take(max_pages) {
            match reply_to
                .channel_id
                .send_message(&ctx, |msg| {
                    msg.allowed_mentions(|men| men.empty_parse())
                        .reference_message(&reply_to)
                        .flags(MessageFlags::SUPPRESS_EMBEDS)
                        .content(chunk)
                })
                .await
            {
                Ok(reply) => {
                    responses.push(reply.clone());
                    reply_to = reply;
                }
                Err(err) => {
                    log::error!("error sending response: {:?}", err);
                }
            }
        }

        Ok(responses)
    }
}
