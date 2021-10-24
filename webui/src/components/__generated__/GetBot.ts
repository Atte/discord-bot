/* tslint:disable */
/* eslint-disable */
// @generated
// This file was automatically generated and should not be edited.

// ====================================================
// GraphQL query operation: GetBot
// ====================================================

export interface GetBot_bot {
  __typename: "User";
  id: string;
  name: string;
  discriminator: number;
  avatar: string | null;
}

export interface GetBot {
  bot: GetBot_bot;
}
