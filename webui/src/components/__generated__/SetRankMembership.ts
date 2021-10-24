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
  current: boolean;
}

export interface SetRankMembership {
  setRankMembership: SetRankMembership_setRankMembership;
}

export interface SetRankMembershipVariables {
  guildId: string;
  rankId: string;
  in: boolean;
}
