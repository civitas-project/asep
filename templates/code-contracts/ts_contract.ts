/**
 * Authenticates an incoming API request using an Ed25519 asymmetric signature.
 *
 * @remarks
 * Invariants:
 * - State is idempotent; repeated calls with identical cryptographic nonces produce identical authorization tokens.
 * - Nonce timestamps must fall within a 300-second drift window relative to server UTC.
 *
 * @param token - Raw authorization header payload in "Bearer <jwt>" format.
 * @param maxClockSkewSeconds - Maximum allowable system clock drift tolerance.
 *
 * @returns Decoded session context containing verified claims and tenant identifier.
 *
 * @throws {AuthenticationError} If the cryptographic signature fails verification or clock skew exceeds threshold.
 * @throws {TokenExpiredError} If token expiration timestamp precedes current server epoch.
 */
export async function authenticateBearerToken(
  token: string,
  maxClockSkewSeconds: number = 300
): Promise<SessionContext> {
  // @impl:FR-AUTH-01
  if (!token.startsWith("Bearer ")) {
    throw new AuthenticationError("Invalid bearer token scheme");
  }
  return verifySignature(token.slice(7), maxClockSkewSeconds);
}

export interface SessionContext {
  readonly tenantId: string;
  readonly userId: string;
  readonly roles: readonly string[];
}

export class AuthenticationError extends Error {}
export class TokenExpiredError extends Error {}

declare function verifySignature(jwt: string, skew: number): Promise<SessionContext>;
