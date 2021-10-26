import { gql, useQuery } from '@apollo/client';
import { createHashHistory } from 'history';
import Router, { Route } from 'preact-router';
import { memo } from 'preact/compat';
import DiscordImage from './DiscordImage';
import Guild from './Guild';
import NavLink from './NavLink';
import Redirect from './Redirect';
import Spinner from './Spinner';
import { GetGuilds } from './__generated__/GetGuilds';

export default memo(Guilds);
function Guilds() {
    const { data, error } = useQuery<GetGuilds>(
        gql`
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
        `,
        { ssr: false },
    );
    const guilds = data?.guilds;

    if (error) {
        console.error(error);
        return null;
    }

    if (!guilds) {
        return <Spinner class="uk-padding-small" ratio={3} />;
    }

    return (
        <div class="uk-padding-small uk-animation-fade uk-animation-fast">
            <ul class="uk-margin-remove-bottom uk-tab" uk-tab>
                {guilds.map((guild) => (
                    <NavLink key={guild.id} path={`/guilds/${encodeURIComponent(guild.name)}`}>
                        {guild.icon && (
                            <DiscordImage type="icon" guild_id={guild.id} guild_icon={guild.icon} size={32} squircle />
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
                <Route default component={Redirect} to={`/guilds/${encodeURIComponent(guilds[0].name)}`} />
            </Router>
        </div>
    );
}
