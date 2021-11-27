import DiscordImage from './DiscordImage';
import { useQuery, gql } from '@apollo/client';
import { GetBot } from './__generated__/GetBot';
import { memo } from 'preact/compat';

const inlineSource = globalThis.document?.head.querySelector<HTMLScriptElement>(
    'script[type="application/x-bot-user+json"]',
);
const inlineBot = inlineSource?.textContent && JSON.parse(inlineSource.textContent);

export default memo(NavbarBot);
function NavbarBot() {
    const { data, error } = inlineBot
        ? { data: { bot: inlineBot }, error: undefined }
        : // eslint-disable-next-line react-hooks/rules-of-hooks -- `inlineBot` is static, so the condition always evaluates the same way
          useQuery<GetBot>(
              gql`
                  query GetBot {
                      bot {
                          id
                          name
                          avatar
                      }
                  }
              `,
          );
    const bot = data?.bot;

    if (error) {
        throw error;
    }

    return (
        <>
            {bot?.avatar && <DiscordImage type="avatar" user_id={bot.id} user_avatar={bot.avatar} size={32} circle />}{' '}
            {bot?.name}
        </>
    );
}
