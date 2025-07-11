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

/**
 * Converts a display name to initials which can be used in an avatar fallback.
 * @param name user's display name
 */
export function nameToInitials(name: string): string {
    let parts = name.split(' ');
    if (parts.length == 0) {
        return '?';
    }
    let first = parts[0];
    let last = parts.length > 1 ? parts[parts.length - 1] : '';
    if (first.length > 0 && last.length > 0) {
        return (first[0] + last[0]).toUpperCase();
    } else if (first.length > 0) {
        return first[0];
    } else if (last.length > 0) {
        return last[0];
    } else {
        return '?';
    }
}
