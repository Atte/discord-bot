import { gql, useMutation } from '@apollo/client';
import { memo } from 'preact/compat';
import Errors from './Errors';
import Spinner from './Spinner';
import { GetGuilds_guilds } from './__generated__/GetGuilds';
import { SetRankMembership, SetRankMembershipVariables } from './__generated__/SetRankMembership';

export default memo(GuildRanks);
function GuildRanks({ guild }: { guild: GetGuilds_guilds }) {
    const [setRankMembership, { loading, error }] = useMutation<SetRankMembership, SetRankMembershipVariables>(
        gql`
            mutation SetRankMembership($guildId: ID!, $rankId: ID!, $in: Boolean!) {
                setRankMembership(guildId: $guildId, rankId: $rankId, in: $in) {
                    id
                    current
                }
            }
        `,
        {
            optimisticResponse(variables) {
                return {
                    setRankMembership: {
                        __typename: 'Rank',
                        id: variables.rankId,
                        current: variables.in,
                    },
                };
            },
        },
    );

    return (
        <>
            <Errors errors={[error]} />
            <form>
                {guild.ranks ? (
                    <ul class="uk-list uk-column-1-2@s uk-column-1-3@m uk-column-1-4@l uk-column-1-5@xl uk-animation-slide-top-small">
                        {guild.ranks.map((rank) => (
                            <li key={rank.id} style="break-inside: avoid">
                                <label style="cursor: pointer">
                                    <input
                                        class="uk-checkbox"
                                        type="checkbox"
                                        disabled={loading}
                                        checked={rank.current}
                                        onChange={() =>
                                            setRankMembership({
                                                variables: {
                                                    guildId: guild.id,
                                                    rankId: rank.id,
                                                    in: !rank.current,
                                                },
                                            })
                                        }
                                    />{' '}
                                    {rank.name}
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
