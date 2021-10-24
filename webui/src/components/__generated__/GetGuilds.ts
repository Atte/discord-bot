/* tslint:disable */
/* eslint-disable */
// @generated
// This file was automatically generated and should not be edited.

// ====================================================
// GraphQL query operation: GetGuilds
// ====================================================

export interface GetGuilds_guilds_ranks {
  __typename: "Rank";
  id: string;
  name: string;
  /**
   * Whether the logged in user currently has this rank or not
   */
  current: boolean;
}

export interface GetGuilds_guilds {
  __typename: "Guild";
  id: string;
  name: string;
  /**
   * Guild icon image ID
   */
  icon: string | null;
  /**
   * Whether the logged in user is an admin of the guild or not
   */
  admin: boolean;
  /**
   * All bot managed ranks in the guild
   */
  ranks: GetGuilds_guilds_ranks[];
}

export interface GetGuilds {
  /**
   * All guilds both the bot and the logged in user are in
   */
  guilds: GetGuilds_guilds[];
}
