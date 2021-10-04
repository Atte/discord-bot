import Router, { Route } from 'preact-router';
import { useErrorBoundary } from 'preact/hooks';
import { createHashHistory } from 'history';
import { useFetch } from '../util';
import DiscordImage from './DiscordImage';
import Redirect from './Redirect';
import Errors from './Errors';
import Spinner from './Spinner';
import { CurrentUserData, GuildData } from '../apitypes';
import Guild from './Guild';
import { NavLink } from './NavLink';

export default function App({ bot }: { bot: CurrentUserData }) {
    const [childError] = useErrorBoundary();
    const [user, userError] = useFetch<CurrentUserData>('api/me/user');
    const [guilds, guildsError] = useFetch<GuildData[]>('api/me/guilds');

    return (
        <div class="uk-container">
            <nav class="uk-navbar-container uk-flex-wrap" uk-navbar>
                <div class="uk-navbar-left">
                    <div class="uk-navbar-item uk-logo">
                        {bot.avatar && (
                            <DiscordImage type="avatar" user_id={bot.id} user_avatar={bot.avatar} size={32} circle />
                        )}{' '}
                        {bot.username}
                    </div>
                    {guilds && (
                        <ul class="uk-navbar-nav uk-animation-fade uk-animation-fast">
                            {guilds.map((guild) => (
                                <NavLink key={guild.id} path={`/guilds/${encodeURIComponent(guild.name)}`}>
                                    {guild.icon && (
                                        <DiscordImage
                                            type="icon"
                                            guild_id={guild.id}
                                            guild_icon={guild.icon}
                                            size={16}
                                            squircle
                                        />
                                    )}{' '}
                                    {guild.name}
                                </NavLink>
                            ))}
                        </ul>
                    )}
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
                            <span class="uk-text-bold">{user.username}</span>#{user.discriminator}
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
            {userError?.message === '404' || guildsError?.message === '404' ? (
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
                        <Router history={createHashHistory()}>
                            {guilds.map((guild) => (
                                <Route
                                    key={guild.id}
                                    path={`/guilds/${encodeURIComponent(guild.name)}/:rest*`}
                                    component={Guild}
                                    guild={guild}
                                />
                            ))}
                            <Route default component={Redirect} to={`/guilds/${encodeURIComponent(guilds[0].name)}`} />
                        </Router>
                    ) : (
                        <Spinner class="uk-padding-small" ratio={3} />
                    )}
                </>
            )}
        </div>
    );
}
