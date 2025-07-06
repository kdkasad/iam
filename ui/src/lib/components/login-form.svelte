<script lang="ts">
	import UserLockIcon from '@lucide/svelte/icons/user-lock';
	import type { HTMLAttributes } from 'svelte/elements';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { cn, type WithElementRef } from '$lib/utils.js';
	import { appConfig } from '$lib/app-config';

	let {
		ref = $bindable(null),
		class: className,
		email = $bindable(''),
		isLoading = false,
		error = $bindable(null),
		onsubmit = undefined,
		...restProps
	}: Omit<WithElementRef<HTMLAttributes<HTMLDivElement>>, 'onsubmit'> & {
		email?: string;
		isLoading?: boolean;
		error?: string | null;
		onsubmit?: Pick<HTMLAttributes<HTMLFormElement>, 'onsubmit'>['onsubmit'];
	} = $props();

	const id = $props.id();
</script>

<div class={cn('flex flex-col gap-6', className)} bind:this={ref} {...restProps}>
	<form {onsubmit}>
		<div class="flex flex-col gap-6">
			<div class="flex flex-col items-center gap-2">
				<div class="flex flex-col items-center gap-2 font-medium">
					<div class="flex size-8 items-center justify-center rounded-md">
						<UserLockIcon class="size-6" />
					</div>
					<span class="sr-only">{$appConfig.instanceName}</span>
				</div>
				<h1 class="text-xl font-bold">Welcome to {$appConfig.instanceName}</h1>
				<div class="text-center text-sm">
					Don&apos;t have an account?
					<a href="/register" class="underline underline-offset-4"> Sign up </a>
				</div>
			</div>
			<div class="flex flex-col gap-6">
				<div class="grid gap-3">
					<Label for="email-{id}">Email</Label>
					<Input
						id="email-{id}"
						type="email"
						name="email"
						placeholder="me@example.com"
						autofocus
						autocomplete="email webauthn"
						required
						bind:value={email}
						disabled={isLoading}
					/>
				</div>

				<div>
					{#if error}
						<p class="text-sm text-red-500 mb-4">{error}</p>
					{/if}

					<Button type="submit" class="w-full" loading={isLoading}>
						{isLoading ? 'Logging in...' : 'Log in'}
					</Button>
				</div>
			</div>
		</div>
	</form>
</div>
