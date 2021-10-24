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
  current: boolean;
}

export interface GetGuilds_guilds {
  __typename: "Guild";
  id: string;
  name: string;
  icon: string | null;
  admin: boolean;
  ranks: GetGuilds_guilds_ranks[];
}

export interface GetGuilds {
  guilds: GetGuilds_guilds[];
}
