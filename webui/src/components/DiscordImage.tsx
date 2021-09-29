import React from 'react';
import { unreachable } from '../util';

interface UserAvatar {
    type: 'avatar';
    user_id: string;
    user_avatar: string;
}

type Image = UserAvatar & {
    animated?: boolean,
    size?: 16 | 32 | 64 | 128 | 256 | 512 | 1024 | 2048 | 4096,
};

export default function DiscordImage(props: Image): JSX.Element {
    const width = props.size || 16;
    const height = props.size || 16;
    let src = `https://cdn.discordapp.com/${props.type}s/`;
    switch (props.type) {
        case 'avatar':
            const hasAnimated = props.user_avatar.startsWith('a_');
            if (props.animated && hasAnimated) {
                src += props.user_id + '/' + props.user_avatar + '.gif';
            } else {
                src += props.user_id + '/' + (hasAnimated ? props.user_avatar.substr(2) : props.user_avatar) + '.png';
            }

            break;
        default:
            unreachable(props.type);
    }
    return <img alt="" width={width} height={height} src={src} />;
}
