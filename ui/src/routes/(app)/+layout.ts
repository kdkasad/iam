import type { Session, User } from "$lib/models";
import type { LayoutLoad } from "./$types";

export const load: LayoutLoad = async ({ fetch }) => {
    let data: {
        user: User;
        session: Session;
    } | undefined;

    let response = await fetch('/api/v1/auth/session', {
        credentials: 'include',
    });
    if (response.ok) {
        data = await response.json();
    } else {
        console.error('Failed to fetch session in layout', response.status, response.statusText, await response.text())
    }

    return data;
};
