<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLElement>> = $props();
</script>

<main
	bind:this={ref}
	data-slot="sidebar-inset"
	class={cn(
		"bg-background relative flex w-full flex-1 flex-col",
		"md:peer-data-[variant=inset]:peer-data-[state=expanded]:m-2 transition-[margin,box-shadow] md:peer-data-[variant=inset]:peer-data-[state=expanded]:rounded-xl md:peer-data-[variant=inset]:peer-data-[state=expanded]:shadow-sm",
		className
	)}
	{...restProps}
>
	{@render children?.()}
</main>
