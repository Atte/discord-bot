import Router, { Route } from 'preact-router';
import { createHashHistory } from 'history';
import GuildRanks from './GuildRanks';
import Redirect from './Redirect';
import NavLink from './NavLink';
import GuildRules from './GuildRules';
import { GetGuilds_guilds } from './__generated__/GetGuilds';
import { memo } from 'preact/compat';

export default memo(Guild);
function Guild({ guild }: { guild: GetGuilds_guilds }) {
    const path = `/guilds/${encodeURIComponent(guild.name)}`;
    return (
        <div class="uk-padding-small uk-animation-fade uk-animation-fast">
            <ul class="uk-tab" uk-tab>
                <NavLink path={`${path}/ranks`}>
                    <span uk-icon="users" /> Ranks
                </NavLink>
                {guild.admin && false && (
                    <NavLink path={`${path}/rules`}>
                        <span uk-icon="file-text" /> Rules
                    </NavLink>
                )}
            </ul>
            <div class="uk-card uk-card-default uk-card-body">
                <Router history={createHashHistory()}>
                    <Route path={`${path}/ranks`} component={GuildRanks} guild={guild} />
                    <Route path={`${path}/rules`} component={GuildRules} guild={guild} />
                    <Route default component={Redirect} to={`${path}/ranks`} />
                </Router>
            </div>
        </div>
    );
}
