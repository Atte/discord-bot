import Router, { Route } from 'preact-router';
import { useEffect, useErrorBoundary } from 'preact/hooks';
import { createHashHistory } from 'history';
import { ResponseStatusError } from '../util';
import DiscordImage from './DiscordImage';
import Redirect from './Redirect';
import Errors from './Errors';
import Spinner from './Spinner';
import Guild from './Guild';
import { NavLink } from './NavLink';
import { useQuery, gql } from '@apollo/client';
import { GetBot_bot } from '../__generated__/GetBot';
import { GetMe } from './__generated__/GetMe';
import { GetGuilds } from './__generated__/GetGuilds';

export default function App({ bot }: { bot: GetBot_bot }) {
    const [childError] = useErrorBoundary();

    const { data: userData, error: userError } = useQuery<GetMe>(gql`
        query GetMe {
            me {
                id
                name
                discriminator
                avatar
            }
        }
    `);
    const user = userData?.me;

    const { data: guildsData, error: guildsError } = useQuery<GetGuilds>(gql`
        query GetGuilds {
            guilds {
                id
                name
                icon
                admin
                ranks {
                    id
                    name
                    current
                }
            }
        }
    `);
    const guilds = guildsData?.guilds;

    useEffect(() => {
        // UIkit only does some additional styling,
        // so only load it after first paint to seem faster
        import('../uikit');
    }, []);

    const needToLogin =
        (userError instanceof ResponseStatusError && userError.response.status === 404) ||
        (guildsError instanceof ResponseStatusError && guildsError.response.status === 404);

    return (
        <div class="uk-container">
            <nav class="uk-navbar-container uk-flex-wrap uk-navbar" uk-navbar>
                <div class="uk-navbar-left">
                    <div class="uk-navbar-item uk-logo">
                        {bot.avatar && (
                            <DiscordImage type="avatar" user_id={bot.id} user_avatar={bot.avatar} size={32} circle />
                        )}{' '}
                        {bot.name}
                    </div>
                </div>
                <div class="uk-navbar-right">
                    {user && (
                        <div class="uk-navbar-item uk-animation-fade uk-animation-fast">
                            {user.avatar && (
                                <DiscordImage
                                    type="avatar"
                                    user_id={user.id}
                                    user_avatar={user.avatar}
                                    size={32}
                                    circle
                                />
                            )}{' '}
                            <span class="uk-text-bold">{user.name}</span>#{user.discriminator}
                        </div>
                    )}
                    {(user || guilds) && (
                        <div class="uk-navbar-item uk-animation-fade uk-animation-fast">
                            <form action="api/auth/clear" method="POST">
                                <button class="uk-button uk-button-primary">
                                    <span uk-icon="sign-out" /> Sign out
                                </button>
                            </form>
                        </div>
                    )}
                </div>
            </nav>
            {needToLogin ? (
                <div class="uk-padding-small">
                    <form action="api/auth/redirect" method="POST">
                        <button class="uk-button uk-button-primary uk-animation-fade uk-animation-fast">
                            <span uk-icon="sign-in" /> Sign in with Discord
                        </button>
                    </form>
                </div>
            ) : (
                <>
                    <Errors errors={[childError, userError, guildsError]}>
                        <form action="api/auth/clear" method="POST">
                            <button class="uk-button uk-button-primary">
                                <span uk-icon="refresh" /> Retry
                            </button>
                        </form>
                    </Errors>
                    {guilds ? (
                        <div class="uk-padding-small uk-animation-fade uk-animation-fast">
                            <ul class="uk-margin-remove-bottom uk-tab" uk-tab>
                                {guilds.map((guild) => (
                                    <NavLink key={guild.id} path={`/guilds/${encodeURIComponent(guild.name)}`}>
                                        {guild.icon && (
                                            <DiscordImage
                                                type="icon"
                                                guild_id={guild.id}
                                                guild_icon={guild.icon}
                                                size={32}
                                                squircle
                                            />
                                        )}{' '}
                                        {guild.name}
                                    </NavLink>
                                ))}
                            </ul>
                            <Router history={createHashHistory()}>
                                {guilds.map((guild) => (
                                    <Route
                                        key={guild.id}
                                        path={`/guilds/${encodeURIComponent(guild.name)}/:rest*`}
                                        component={Guild}
                                        guild={guild}
                                    />
                                ))}
                                <Route
                                    default
                                    component={Redirect}
                                    to={`/guilds/${encodeURIComponent(guilds[0].name)}`}
                                />
                            </Router>
                        </div>
                    ) : (
                        <Spinner class="uk-padding-small" ratio={3} />
                    )}
                </>
            )}
        </div>
    );
}
