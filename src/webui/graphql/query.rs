use super::{
    guilds::{guild_member, guild_roles, guild_rules, ranks_from_guild},
    Context,
};
use futures::stream::{self, StreamExt};
use juniper::{FieldResult, ID};
use juniper_codegen::graphql_object;
use log::trace;
use serenity::model::{
    channel::Message,
    guild::{GuildInfo, Role},
    id::{GuildId, RoleId},
    permissions::Permissions,
    user::CurrentUser,
};
use std::borrow::Cow;

pub mod types {
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

    pub struct Rank {
        pub role: Role,
        pub current: Option<bool>,
    }

    #[graphql_object(Context = Context)]
    impl Rank {
        fn id(&self) -> ID {
            ID::new(self.role.id.0.to_string())
        }

        fn name(&self) -> &str {
            &self.role.name
        }

        async fn current(&self, context: &Context) -> FieldResult<bool> {
            if let Some(current) = self.current {
                return Ok(current);
            }

            let member = guild_member(
                self.role.guild_id,
                context.user.as_ref().ok_or("unauthenticated")?.id,
                &context.webui.discord,
            )
            .await
            .ok_or("can't find member")?;
            Ok(member.roles.contains(&self.role.id))
        }
    }

    pub struct Rules(pub Message);

    #[graphql_object(Context = Context)]
    impl Rules {
        fn id(&self) -> ID {
            ID::new(self.0.id.0.to_string())
        }

        fn content(&self) -> &str {
            &self.0.content
        }
    }

    pub struct Guild(pub GuildInfo);

    #[graphql_object(Context = Context)]
    impl Guild {
        fn id(&self) -> ID {
            ID::new(self.0.id.0.to_string())
        }

        fn name(&self) -> &str {
            &self.0.name
        }

        fn icon(&self) -> &Option<String> {
            &self.0.icon
        }

        async fn admin(&self, context: &Context) -> FieldResult<bool> {
            let member = guild_member(
                self.0.id,
                context.user.as_ref().ok_or("unauthenticated")?.id,
                &context.webui.discord,
            )
            .await
            .ok_or("can't find member")?;
            for role in guild_roles(self.0.id, &context.webui.discord)
                .await
                .ok_or("can't find guild roles")?
                .values()
            {
                if role.has_permission(Permissions::ADMINISTRATOR)
                    && member.roles.contains(&role.id)
                {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        async fn ranks(&self, context: &Context) -> FieldResult<Vec<Rank>> {
            let mut ranks: Vec<_> = ranks_from_guild(self.0.id, &context.webui.discord)
                .await?
                .map(|role| Rank {
                    role,
                    current: None,
                })
                .collect();
            ranks.sort_by(|a, b| a.role.name.cmp(&b.role.name));
            Ok(ranks)
        }

        async fn rules(&self, context: &Context) -> Option<Rules> {
            guild_rules(self.0.id, &context.webui.discord, &context.webui.config)
                .await
                .map(Rules)
        }
    }
}

pub struct Query;

#[graphql_object(Context = Context)]
impl Query {
    async fn bot(context: &Context) -> types::User {
        types::User(Cow::Owned(context.webui.discord.cache.current_user().await))
    }

    fn me(context: &Context) -> FieldResult<types::User> {
        Ok(types::User(Cow::Borrowed(
            context.user.as_ref().ok_or("unauthenticated")?,
        )))
    }

    async fn guild(context: &Context, id: ID) -> FieldResult<types::Guild> {
        let info = context
            .webui
            .guilds
            .get(&GuildId(id.parse()?))
            .ok_or("not a bot guild")?;
        guild_member(
            info.id,
            context.user.as_ref().ok_or("unauthenticated")?.id,
            &context.webui.discord,
        )
        .await
        .ok_or("can't find member")?;
        Ok(types::Guild(info.clone()))
    }

    async fn guilds(context: &Context) -> FieldResult<Vec<types::Guild>> {
        let user_id = context.user.as_ref().ok_or("unauthenticated")?.id;
        let mut guilds: Vec<_> = stream::iter(context.webui.guilds.values())
            .filter_map(|info| async move {
                guild_member(info.id, user_id, &context.webui.discord).await?;
                Some(types::Guild(info.clone()))
            })
            .collect()
            .await;
        guilds.sort_by(|a, b| a.0.name.cmp(&b.0.name));
        Ok(guilds)
    }
}

pub struct Mutation;

#[graphql_object(Context = Context)]
impl Mutation {
    async fn set_rank_membership(
        context: &Context,
        guild_id: ID,
        rank_id: ID,
        r#in: bool,
    ) -> FieldResult<types::Rank> {
        let guild_id = GuildId(guild_id.parse()?);
        let rank_id = RoleId(rank_id.parse()?);

        let user = context.user.as_ref().ok_or("unauthenticated")?;
        let mut member = guild_member(guild_id, user.id, &context.webui.discord)
            .await
            .ok_or("can't find member")?;

        let role = ranks_from_guild(guild_id, &context.webui.discord)
            .await?
            .find(|role| role.id == rank_id)
            .ok_or("invalid rank")?;

        if r#in {
            member.add_role(&context.webui.discord.http, &role).await?;
            trace!(
                "{} ({}) joined rank {} ({})",
                user.tag(),
                user.id,
                role.name,
                role.id
            );
        } else {
            member
                .remove_role(&context.webui.discord.http, &role)
                .await?;
            trace!(
                "{} ({}) left rank {} ({})",
                user.tag(),
                user.id,
                role.name,
                role.id
            );
        }

        Ok(types::Rank {
            role,
            current: Some(r#in),
        })
    }
}
