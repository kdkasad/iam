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
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();

	const id = $props.id();
</script>

<div class={cn('flex flex-col gap-6', className)} bind:this={ref} {...restProps}>
	<form>
		<div class="flex flex-col gap-6">
			<div class="flex flex-col items-center gap-2">
				<a href="##" class="flex flex-col items-center gap-2 font-medium">
					<div class="flex size-8 items-center justify-center rounded-md">
						<UserLockIcon class="size-6" />
					</div>
					<span class="sr-only">{$appConfig.instanceName}</span>
				</a>
				<h1 class="text-xl font-bold">Welcome to {$appConfig.instanceName}</h1>
				<div class="text-center text-sm">
					Don&apos;t have an account?
					<a href="##" class="underline underline-offset-4"> Sign up </a>
				</div>
			</div>
			<div class="flex flex-col gap-6">
				<div class="grid gap-3">
					<Label for="email-{id}">Email</Label>
					<Input
						id="email-{id}"
						type="email"
						placeholder="me@example.com"
						autofocus
						autocomplete="email webauthn"
						required
					/>
				</div>
				<Button type="submit" class="w-full">Log in</Button>
			</div>
		</div>
	</form>
</div>
