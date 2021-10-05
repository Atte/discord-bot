import { unreachable, useMediaQuery } from '../util';

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

export default function DiscordImage(props: DiscordImageProps) {
    const reducedMotion = useMediaQuery('(prefers-reduced-motion)');

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

    const animate = props.animated && ids[ids.length - 1].startsWith('a_') && !reducedMotion;
    const borderRadius = Math.ceil(props.circle ? props.size / 2 : props.squircle ? props.size / 3 : 0);
    const baseUrl = `https://cdn.discordapp.com/${props.type}s/${ids.join('/')}`;

    return (
        <picture>
            {!animate && <source srcset={`${baseUrl}.webp?size=${props.size}`} type="image/webp" />}
            <img
                alt=""
                width={props.size}
                height={props.size}
                src={`${baseUrl}.${animate ? 'gif' : 'png'}?size=${props.size}`}
                style={borderRadius > 0 ? `border-radius: ${borderRadius}px` : undefined}
                crossOrigin="anonymous"
            />
        </picture>
    );
}
