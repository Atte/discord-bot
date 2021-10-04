export interface CurrentUserData {
    id: string;
    avatar?: string;
    bot: boolean;
    discriminator: number;
    email?: string;
    mfa_enabled: boolean;
    username: string;
    verified?: boolean;
    public_flags?: number;
}

export interface GuildData {
    id: string;
    icon?: string;
    name: string;
    admin: boolean;
    ranks: {
        current: RoleData[];
        available: RoleData[];
    };
}

export interface RoleData {
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
        bot_id?: string;
        integration_id?: string;
        premium_subscriber: boolean;
    };
}
