<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import type { AppConfig, Session } from '$lib/models';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import { getContext, onMount } from 'svelte';

	let { instanceName } = getContext<AppConfig>('appConfig');
    let getSession: (() => Session) | undefined = getContext<() => Session>('session');

    let loading = $state(false);

    // If already logged out, go directly to login page
    onMount(() => {
        console.debug('getSession', getSession, getSession?.());
        if (!getSession?.()) {
            goto('/login', { invalidateAll: true });
        }
    })

    const onsubmit = async (event: SubmitEvent) => {
        event.preventDefault();
        loading = true;
        const response = await fetch('/api/v1/logout', {
            method: 'POST',
            credentials: 'include'
        });
        loading = false;
        if (!response.ok) {
            console.error('Failed to log out:', response.status, response.statusText, await response.text());
            return;
        }
        // Redirect to login page
        goto('/login', { invalidateAll: true });
    };
</script>

<div class="flex h-full flex-col items-center justify-center gap-6">
	<div class="w-full max-w-sm">
		<form method="post" {onsubmit}>
			<div class="flex flex-col gap-6">
				<div class="flex flex-col items-center gap-2">
					<div class="flex flex-col items-center gap-2 font-medium">
						<div class="flex size-8 items-center justify-center rounded-md">
							<LogOutIcon class="size-6" />
						</div>
						<span class="sr-only">{instanceName}</span>
					</div>
					<h1 class="text-xl font-bold">
						Log out of {instanceName}
					</h1>
					<div class="text-center text-sm">
						This will log you out of {instanceName}, but may not log you out of any sites
						you're signed in to via SSO.
					</div>
					<Button type="submit" class="w-full mt-8" {loading}>Log out</Button>
				</div>
			</div>
		</form>
	</div>
</div>
