use super::Context;
use juniper::ID;
use juniper_codegen::graphql_object;
use serenity::model::{guild::GuildInfo, id::GuildId, user::CurrentUser};
use std::borrow::Cow;

mod types {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub struct User<'a>(pub Cow<'a, CurrentUser>);

    #[graphql_object]
    impl<'a> User<'a> {
        fn id(&self) -> ID {
            ID::new(self.0.id.0.to_string())
        }

        fn name(&self) -> &str {
            &self.0.name
        }

        fn discriminator(&self) -> i32 {
            i32::from(self.0.discriminator)
        }

        fn avatar(&self) -> &Option<String> {
            &self.0.avatar
        }
    }

    pub struct Guild<'a>(pub Cow<'a, GuildInfo>);

    #[graphql_object]
    impl<'a> Guild<'a> {
        fn id(&self) -> ID {
            ID::new(self.0.id.0.to_string())
        }
    }
}

pub struct Query;

#[graphql_object(
    Context = Context,
)]
impl Query {
    fn me(context: &Context) -> Option<types::User> {
        Some(types::User(Cow::Borrowed(context.user.as_ref()?)))
    }

    async fn bot(context: &Context) -> types::User {
        types::User(Cow::Owned(context.webui.discord.cache.current_user().await))
    }

    async fn guild(context: &Context, id: ID) -> Option<types::Guild> {
        Some(types::Guild(Cow::Borrowed(
            context
                .webui
                .guilds
                .get(&GuildId(id.parse().ok()?))
                .as_ref()?,
        )))
    }

    async fn guilds(context: &Context) -> Vec<types::Guild> {
        context
            .webui
            .guilds
            .values()
            .map(|info| types::Guild(Cow::Borrowed(info)))
            .collect()
    }
}
