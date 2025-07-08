<script lang="ts">
	import type { Component } from "@lucide/svelte";
	import * as Sidebar from "$lib/components/ui/sidebar";

    let { items }: {
		items: {
			title: string;
			url: string;
			icon: Component;
		}[];
	} = $props();
</script>

<Sidebar.Group>

</Sidebar.Group>
