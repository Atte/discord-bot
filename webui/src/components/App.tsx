import Router, { Route } from 'preact-router';
import { Match } from 'preact-router/match';
import { useEffect, useErrorBoundary, useState } from 'preact/hooks';
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

function clearStorage(): void {
    window.location.href = 'auth/clear';
}

function NavLink(props: { path: string, children: any }) {
    return <Match path={props.path}>{({matches}: {matches: boolean}) => 
        <li class={matches ? 'uk-active' : undefined}>
            <a href={props.path}>{props.children}</a>
        </li>
    }</Match>
}

export default function App() {
    const [childError] = useErrorBoundary();
    const [user, userError] = useFetch<CurrentUserData>('me/user');
    const [bot, botError] = useFetch<CurrentUserData>('bot/user');

    useEffect(() => {
        if (userError?.message === '404') {
            window.location.href = 'auth/redirect';
        }
    }, [userError]);

    useEffect(() => {
        if (bot?.username) {
            document.title = bot.username;
        }
    }, [bot]);

    return <div class="uk-container">
        <nav class="uk-navbar-container" uk-navbar>
            <div class="uk-navbar-left">
                <div class="uk-navbar-item uk-logo">
                    {bot?.avatar && <DiscordImage type="avatar" user_id={bot.id} user_avatar={bot.avatar} size={32} circle />}
                    {' '}{bot?.username}
                </div>
                <ul class="uk-navbar-nav">
                    <NavLink path="/ranks"><span uk-icon="users"></span>{' '}Ranks</NavLink>
                </ul>
            </div>
            <div class="uk-navbar-right">
                <div class="uk-navbar-item">
                    {user?.avatar && <DiscordImage type="avatar" user_id={user.id} user_avatar={user.avatar} size={32} circle />}
                    {' '}<span class="uk-text-bold">{user?.username}</span>#{user?.discriminator}
                </div>
                <button class="uk-button uk-button-primary uk-margin-right" onClick={clearStorage}>
                    <span uk-icon="sign-out"></span>
                    {' '}Log out
                </button>
            </div>
        </nav>
        <Errors errors={[childError, userError, botError]} />
        <Router history={createHashHistory()}>
            <Route path="/ranks" component={Guilds} />
            <Route path="/" component={Redirect} to="/ranks" />
        </Router>
    </div>;
}
