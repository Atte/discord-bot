import Router, { Route } from 'preact-router';
import { Match } from 'preact-router/match';
import { useErrorBoundary } from 'preact/hooks';
import { createHashHistory } from 'history';
import { useFetch } from '../util';
import DiscordImage from './DiscordImage';
import Guilds from './Guilds';
import Redirect from './Redirect';
import Errors from './Errors';

export interface CurrentUserData {
    id: string;
    avatar?: string;
    bot: boolean;
    discriminator: number;
    email?: string;
    mfa_enabled: boolean;
    username: string;
    verified?: boolean;
    public_flags?: number;
}

function NavLink(props: { path: string, children: any }) {
    return <Match path={props.path}>{({matches}: {matches: boolean}) => 
        <li class={matches ? 'uk-active' : undefined}>
            <a href={props.path}>{props.children}</a>
        </li>
    }</Match>
}

export default function App({ bot }: { bot: CurrentUserData }) {
    const [childError] = useErrorBoundary();
    const [user, userError] = useFetch<CurrentUserData>('me/user');

    return <div class="uk-container">
        <nav class="uk-navbar-container uk-flex-wrap" uk-navbar>
            <div class="uk-navbar-left">
                <div class="uk-navbar-item uk-logo">
                    {bot.avatar && <DiscordImage type="avatar" user_id={bot.id} user_avatar={bot.avatar} size={32} circle />}
                    {' '}{bot.username}
                </div>
                {user && <ul class="uk-navbar-nav uk-animation-fade uk-animation-fast">
                    <NavLink path="/ranks"><span uk-icon="users" />{' '}Ranks</NavLink>
                </ul>}
            </div>
            {user && <div class="uk-navbar-right uk-animation-fade uk-animation-fast">
                <div class="uk-navbar-item">
                    {user.avatar && <DiscordImage type="avatar" user_id={user.id} user_avatar={user.avatar} size={32} circle />}
                    {' '}<span class="uk-text-bold">{user.username}</span>#{user.discriminator}
                </div>
                <div class="uk-navbar-item">
                    <form action="auth/clear" method="POST">
                        <button class="uk-button uk-button-primary">
                            <span uk-icon="sign-out" />
                            {' '}Log out
                        </button>
                    </form>
                </div>
            </div>}
        </nav>
        {userError?.message === '404'
            ? <form action="auth/redirect" method="POST">
                <button class="uk-button uk-button-primary uk-animation-fade uk-animation-fast">
                    <span uk-icon="sign-in" />
                    {' '}Log in with Discord
                </button>
            </form>
            : <>
                <Errors errors={[childError, userError]} />
                {user ?
                    <Router history={createHashHistory()}>
                        <Route path="/ranks" component={Guilds} />
                        <Route path="/" component={Redirect} to="/ranks" />
                    </Router>
                : <div><div uk-spinner="ratio: 3" /></div>}
            </>
        }
    </div>;
}
