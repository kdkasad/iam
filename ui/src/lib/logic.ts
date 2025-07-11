import type { Session, User } from "./models";

/**
 * Checks if the current user can upgrade to admin privileges.
 * @param user Current user
 * @param session Current session
 * @returns Whether the current user can upgrade to admin privileges
 */
export function canUpgrade(user: User, session: Session): boolean {
    console.log(user);
    return !session.isAdmin && user.tags?.some(tag => tag.name === 'iam::admin') === true;
}
