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
  /**
   * Part of username before the #
   */
  name: string;
  /**
   * Avatar image ID
   */
  avatar: string | null;
}

export interface GetBot {
  /**
   * The bot's Discord user
   */
  bot: GetBot_bot;
}
