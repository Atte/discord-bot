import { sortBy, useFetch } from '../util';
import DiscordImage from './DiscordImage';
import Errors from './Errors';
import GuildRanks from './GuildRanks';

export interface GuildData {
    id: string;
    icon?: string;
    name: string;
    owner: boolean;
    permissions: number | string;
}

export default function Guilds() {
    const [guilds, guildsError] = useFetch<GuildData[]>('api/me/guilds');

    return <>
        <Errors errors={[guildsError]} />
        <div class="uk-flex uk-flex-wrap" uk-flex>
            {guilds ? guilds.sort(sortBy('name')).map(guild => <div key={guild.id} class="uk-padding-small">
                <div class="uk-card uk-card-default uk-card-body uk-animation-fade uk-animation-fast">
                    <h3 class="uk-card-title">
                        {guild.icon && <DiscordImage type="icon" guild_id={guild.id} guild_icon={guild.icon} size={32} squircle />}
                        {' '}{guild.name}
                    </h3>
                    <GuildRanks guild={guild} />
                </div>
            </div>) : <div class="uk-padding-small"><div uk-spinner="ratio: 3" /></div>}
        </div>
    </>;
}
