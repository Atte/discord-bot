/* tslint:disable */
/* eslint-disable */
// @generated
// This file was automatically generated and should not be edited.

// ====================================================
// GraphQL mutation operation: SetRankMembership
// ====================================================

export interface SetRankMembership_setRankMembership {
  __typename: "Rank";
  id: string;
  /**
   * Whether the logged in user currently has this rank or not
   */
  current: boolean;
}

export interface SetRankMembership {
  /**
   * Ensure the logged in user either has or doesn't have the specified rank
   */
  setRankMembership: SetRankMembership_setRankMembership;
}

export interface SetRankMembershipVariables {
  guildId: string;
  rankId: string;
  in: boolean;
}
