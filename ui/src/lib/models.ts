export type Uuid = string;

export interface User {
    id: Uuid;
    email: string;
    displayName: string;
    createdAt: string; // FIXME: use a date type
    updatedAt: string; // FIXME: use a date type
    tags?: any[]; // FIXME: use proper type
    passkeys?: any[]; // FIXME: use proper type
}

export type SessionState =
    | 'active'
    | 'revoked'
    | 'logged-out'
    | 'superseded';

export interface Session {
    state: SessionState;
    createdAt: string;
    expiresAt: string;
    isAdmin: boolean;
}
