import { Fragment } from 'preact';
import { useFetch } from '../util';
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
    const [guilds, guildsError] = useFetch<GuildData[]>('me/guilds');

    return <>
        <Errors errors={[guildsError]} />
        {(guilds ?? []).map(guild => <Fragment key={guild.id}>
            <h3>
                {guild.icon && <DiscordImage type="icon" guild_id={guild.id} guild_icon={guild.icon} size={32} squircle />}
                {' '}{guild.name}
            </h3>
            <GuildRanks guild={guild} />
        </Fragment>)}
    </>;
}
