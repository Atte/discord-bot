import { unreachable } from '../util';

interface UserAvatar {
    type: 'avatar';
    user_id: string;
    user_avatar: string;
}

interface GuildIcon {
    type: 'icon';
    guild_id: string;
    guild_icon: string;
}

type DiscordImageProps = (UserAvatar | GuildIcon) & {
    animated?: boolean;
    circle?: boolean;
    squircle?: boolean;
    size: 16 | 32 | 64 | 128 | 256 | 512 | 1024 | 2048 | 4096;
};

const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

export default function DiscordImage(props: DiscordImageProps) {
    let ids: string[];
    switch (props.type) {
        case 'avatar':
            ids = [props.user_id, props.user_avatar];
            break;
        case 'icon':
            ids = [props.guild_id, props.guild_icon];
            break;
        default:
            unreachable(props);
    }

    const animate = props.animated && ids[ids.length - 1].startsWith('a_') && !reducedMotion.matches;
    const borderRadius = props.circle ? props.size / 2 : props.squircle ? props.size / 3 : 0;

    return (
        <img
            alt=""
            width={props.size}
            height={props.size}
            src={`https://cdn.discordapp.com/${props.type}s/${ids.join('/')}.${animate ? 'gif' : 'webp'}`}
            style={borderRadius > 0 ? `border-radius: ${borderRadius}px` : undefined}
            crossOrigin="anonymous"
        />
    );
}
