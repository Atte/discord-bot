use serenity::all::{
    standard::{macros::command, Args, CommandResult},
    ButtonStyle, Context, CreateAllowedMentions, CreateButton, CreateMessage, Message,
};

#[command]
#[owners_only]
#[help_available(false)]
#[num_args(0)]
async fn test(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.channel_id
        .send_message(
            ctx,
            CreateMessage::new()
                .reference_message(msg)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .button(
                    CreateButton::new("orange")
                        .label("Orange")
                        .emoji('🍊')
                        .style(ButtonStyle::Secondary),
                )
                .button(
                    CreateButton::new("apple")
                        .label("Apple")
                        .emoji('🍎')
                        .style(ButtonStyle::Secondary),
                ),
        )
        .await?;
    Ok(())
}
