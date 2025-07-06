<script>
	import { appConfig } from '$lib/app-config';
	import { Button } from '$lib/components/ui/button';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
</script>

<div class="bg-background flex min-h-svh flex-col items-center justify-center gap-6 p-6 md:p-10">
	<div class="w-full max-w-sm">
		<form method="post" action="/api/v1/logout?next=/login">
			<div class="flex flex-col gap-6">
				<div class="flex flex-col items-center gap-2">
					<div class="flex flex-col items-center gap-2 font-medium">
						<div class="flex size-8 items-center justify-center rounded-md">
							<LogOutIcon class="size-6" />
						</div>
						<span class="sr-only">{$appConfig.instanceName}</span>
					</div>
					<h1 class="text-xl font-bold">
						Log out of {$appConfig.instanceName}
					</h1>
					<div class="text-center text-sm">
						This will log you out of {$appConfig.instanceName}, but may not log you out of any sites
						you're signed in to via SSO.
					</div>
					<Button type="submit" class="w-full mt-8">Log out</Button>
				</div>
			</div>
		</form>
	</div>
</div>
