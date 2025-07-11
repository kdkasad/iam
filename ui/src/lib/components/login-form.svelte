<script lang="ts">
	import UserPlusIcon from '@lucide/svelte/icons/user-plus';
	import UserLockIcon from '@lucide/svelte/icons/user-lock';
	import type { HTMLAttributes } from 'svelte/elements';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { cn, type WithElementRef } from '$lib/utils.js';
	import { AtSignIcon, IdCardIcon } from '@lucide/svelte';
	import { getContext } from 'svelte';
	import type { AppConfig } from '$lib/models';

	let {
		ref = $bindable(null),
		class: className,
		isLoading = false,
		error = $bindable(null),
		onsubmit = undefined,
		register = false,
		...restProps
	}: Omit<WithElementRef<HTMLAttributes<HTMLDivElement>>, 'onsubmit'> & {
		isLoading?: boolean;
		error?: string | null;
		register?: boolean;
		onsubmit?: Pick<HTMLAttributes<HTMLFormElement>, 'onsubmit'>['onsubmit'];
	} = $props();

	let appConfig = getContext<AppConfig>('appConfig');

	const id = $props.id();
</script>

<div class={cn('flex flex-col gap-6', className)} bind:this={ref} {...restProps}>
	<form {onsubmit}>
		<div class="flex flex-col gap-6">
			<div class="flex flex-col items-center gap-2">
				<div class="flex flex-col items-center gap-2 font-medium">
					<div class="flex size-8 items-center justify-center rounded-md">
						{#if register}
							<UserPlusIcon class="size-6" />
						{:else}
							<UserLockIcon class="size-6" />
						{/if}
					</div>
					<span class="sr-only">{appConfig.instanceName}</span>
				</div>
				<h1 class="text-xl font-bold">
					{register ? 'Create an account for' : 'Log in to'}
					{appConfig.instanceName}
				</h1>
				<div class="text-center text-sm">
					{#if register}
						Already have an account?
						<a href="/login" class="underline underline-offset-4"> Log in </a>
					{:else}
						Don&apos;t have an account?
						<a href="/register" class="underline underline-offset-4"> Sign up </a>
					{/if}
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
						disabled={isLoading}
						icon={AtSignIcon}
					/>
				</div>

				{#if register}
					<div class="grid gap-3">
						<Label for="displayName-{id}">Name</Label>
						<Input
							id="displayName-{id}"
							type="text"
							name="displayName"
							placeholder="Your Name"
							autocomplete="name"
							required
							disabled={isLoading}
							icon={IdCardIcon}
						/>
					</div>
				{/if}

				<div>
					{#if error}
						<p class="mb-4 text-sm text-red-500">{error}</p>
					{/if}

					<Button type="submit" class="w-full" loading={isLoading}>
						{#if register}
							{isLoading ? 'Signing up...' : 'Sign up'}
						{:else}
							{isLoading ? 'Logging in...' : 'Log in'}
						{/if}
					</Button>
				</div>
			</div>
		</div>
	</form>
</div>
