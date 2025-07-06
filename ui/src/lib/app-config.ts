import { writable } from "svelte/store";

export interface AppConfig {
    instanceName: string;
}

export const appConfig = writable<AppConfig>({
    instanceName: typeof window !== 'undefined' ? window.location.hostname : 'IAM Server',
});
