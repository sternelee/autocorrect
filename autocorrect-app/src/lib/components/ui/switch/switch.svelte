<script lang="ts" context="module">
	import type { HTMLButtonAttributes } from "svelte/elements";
	import type { WithElementRef } from "$lib/utils.js";

	export type SwitchVariant = "default" | "outline";
	export type SwitchProps = WithElementRef<HTMLButtonAttributes> & {
		checked?: boolean;
		variant?: SwitchVariant;
		class?: string;
		className?: string;
		id?: string;
		onchange?: (e: Event) => void;
		title?: string;
	};
</script>

<script lang="ts">
	import { cn } from "$lib/utils.js";

	export let ref: HTMLButtonElement | undefined = undefined;
	export let checked = false;
	export let variant: SwitchVariant = "default";
	export let className: string | undefined = undefined;
	export let disabled: boolean | undefined = undefined;
	export let id: string | undefined = undefined;
	export let onchange: ((e: Event) => void) | undefined = undefined;
	export let title: string | undefined = undefined;

	let restProps: Partial<Omit<SwitchProps, 'ref' | 'checked' | 'variant' | 'class' | 'className' | 'disabled' | 'id' | 'onchange' | 'title'>> = {};

	function toggle() {
		if (!disabled) {
			checked = !checked;
			// Call onchange if provided
			if (onchange) {
				onchange(new Event('change'));
			}
		}
	}

	const variantClassMap: Record<SwitchVariant, string> = {
		default: "",
		outline: "border-input"
	};
</script>

<button
	bind:this={ref}
	type="button"
	role="switch"
	aria-checked={checked}
	{id}
	{title}
	{disabled}
	onclick={toggle}
	class={cn(
		"relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
		variantClassMap[variant],
		checked ? "bg-primary" : "bg-input",
		disabled && "cursor-not-allowed opacity-50",
		className
	)}
	{...restProps}
>
	<span
		class={cn(
			"pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-lg ring-0 transition-transform duration-200 ease-in-out",
			checked ? "translate-x-4" : "translate-x-0"
		)}
	></span>
</button>
