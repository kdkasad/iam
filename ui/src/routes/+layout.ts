import type { AppConfig } from '$lib/models';
import type { LayoutLoad } from './$types';

/**
 * Fetches the server config from the API and returns it in the `appConfig` property.
 * If the fetch fails, the domain name is used as the fallback.
 */
export const load: LayoutLoad = async ({ fetch }) => {
    let appConfig: AppConfig | undefined;
    try {
        const response = await fetch('/api/v1/config');
        if (response.ok) {
            appConfig = await response.json() satisfies AppConfig;
        } else {
            console.error("Failed to fetch server config in load function:", response.statusText);
        }
    } catch (error) {
        console.error("Failed to fetch server config in load function:", error);

        // Fall back to the domain name
        appConfig = {
            instanceName: window.location.hostname
        };
    }

    return {
        appConfig: appConfig,
    };
};
