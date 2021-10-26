import { useQuery } from '@apollo/client';
import gql from 'graphql-tag';
import { memo } from 'preact/compat';
import DiscordImage from './DiscordImage';
import { GetMe } from './__generated__/GetMe';

export default memo(NavbarUser);
function NavbarUser() {
    const { data, error } = useQuery<GetMe>(
        gql`
            query GetMe {
                me {
                    id
                    name
                    discriminator
                    avatar
                }
            }
        `,
        { ssr: false },
    );
    const user = data?.me;

    if (error) {
        throw error;
    }

    if (!user) {
        return null;
    }

    return (
        <>
            <div class="uk-navbar-item uk-animation-fade uk-animation-fast">
                {user.avatar && (
                    <DiscordImage type="avatar" user_id={user.id} user_avatar={user.avatar} size={32} circle />
                )}{' '}
                <span class="uk-text-bold">{user.name}</span>#{user.discriminator.toString().padStart(4, '0')}
            </div>

            <div class="uk-navbar-item uk-animation-fade uk-animation-fast">
                <form action="api/auth/clear" method="POST">
                    <button class="uk-button uk-button-primary">
                        <span uk-icon="sign-out" /> Sign out
                    </button>
                </form>
            </div>
        </>
    );
}
