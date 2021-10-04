import { useState } from 'preact/hooks';
import { GuildData, RoleData } from '../apitypes';
import { sortByProp } from '../util';
import Errors from './Errors';
import Spinner from './Spinner';

export default function GuildRanks({ guild }: { guild: GuildData }) {
    const [ranks, setRanks] = useState(guild.ranks);
    const [ranksError, setRanksError] = useState<Error | undefined>(undefined);
    const [changing, setChanging] = useState(false);

    async function setRole(role: RoleData, on: boolean): Promise<void> {
        setChanging(true);
        try {
            const response = await fetch(`api/me/guilds/${role.guild_id}/ranks/${role.id}`, {
                method: on ? 'POST' : 'DELETE',
            });
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

    return (
        <>
            <Errors errors={[ranksError]} />
            <form>
                {ranks ? (
                    <ul class="uk-list uk-column-1-2@s uk-animation-slide-top-small">
                        {ranks.current
                            .concat(ranks.available)
                            .sort(sortByProp('name'))
                            .map((role) => (
                                <li key={role.id} style="break-inside: avoid">
                                    <label style="cursor: pointer">
                                        <input
                                            class="uk-checkbox"
                                            type="checkbox"
                                            disabled={changing}
                                            checked={ranks.current.includes(role)}
                                            onChange={() => setRole(role, !ranks.current.includes(role))}
                                        />{' '}
                                        {role.name}
                                    </label>
                                </li>
                            ))}
                    </ul>
                ) : (
                    <Spinner class="uk-text-center" />
                )}
            </form>
        </>
    );
}
