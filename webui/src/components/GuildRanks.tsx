import { useState } from 'preact/hooks';
import { useFetch } from '../util';
import Errors from './Errors';
import { GuildData } from './Guilds';

interface Role {
    id: string;
    guild_id: string;
    color: number;
    hoist: boolean;
    managed: boolean;
    mentionable: boolean;
    name: string;
    permissions: number | string;
    position: number;
    tags: {
        bot_id?: string,
        integration_id?: string,
        premium_subscriber: boolean,
    };
}

interface GuildRanks {
    current: Role[];
    available: Role[];
}

export default function GuildRanks(props: { guild: GuildData }) {
    const [ranks, ranksError, setRanks, setRanksError] = useFetch<GuildRanks>(`me/guilds/${props.guild.id}/ranks`);
    const [changing, setChanging] = useState(false);

    async function setRole(role: Role, on: boolean): Promise<void> {
        setChanging(true);
        try {
            const response = await fetch(`me/guilds/${role.guild_id}/ranks/${role.id}`, { method: on ? 'POST' : 'DELETE' });
            if (!response.ok) {
                throw new Error(response.statusText);
            }
            setRanks(await response.json());
        } catch (err) {
            setRanksError(err as Error);
        } finally {
            setChanging(false);
        }
    }

    return <>
        <Errors errors={[ranksError]} />
        <form>
            <ul class="uk-list">
                {ranks && ranks.current.concat(ranks.available).sort((a, b) => a.name.localeCompare(b.name)).map(role =>
                    <li key={role.id}>
                        <label>
                            <input
                                class="uk-checkbox"
                                type="checkbox"
                                disabled={changing}
                                checked={ranks.current.includes(role)}
                                onChange={() => setRole(role, !ranks.current.includes(role))}
                            />
                            {' '}{role.name}
                        </label>
                    </li>
                )}
            </ul>
        </form>
    </>;
}
