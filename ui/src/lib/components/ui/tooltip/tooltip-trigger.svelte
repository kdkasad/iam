<script lang="ts">
	import { cn } from "$lib/utils";
	import { Tooltip as TooltipPrimitive } from "bits-ui";

	let { ref = $bindable(null), class: className, ...restProps }: TooltipPrimitive.TriggerProps = $props();
</script>

<TooltipPrimitive.Trigger bind:ref data-slot="tooltip-trigger" class={cn('underline decoration-dotted', className)} {...restProps} />
