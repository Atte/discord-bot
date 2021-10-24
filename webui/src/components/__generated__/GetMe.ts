/* tslint:disable */
/* eslint-disable */
// @generated
// This file was automatically generated and should not be edited.

// ====================================================
// GraphQL query operation: GetMe
// ====================================================

export interface GetMe_me {
  __typename: "User";
  id: string;
  /**
   * Part of username before the #
   */
  name: string;
  /**
   * Part of username after the #
   */
  discriminator: number;
  /**
   * Avatar image ID
   */
  avatar: string | null;
}

export interface GetMe {
  /**
   * The logged in user's Discord user
   */
  me: GetMe_me;
}
