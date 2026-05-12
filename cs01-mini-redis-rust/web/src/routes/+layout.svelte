<script lang="ts">
	import { page } from '$app/stores';
	import '../app.css';

	let { children } = $props();

	const links = [
		{ href: '/', label: 'Dashboard' },
		{ href: '/keys', label: 'Keys' },
		{ href: '/pubsub', label: 'Pub/Sub' }
	] as const;
</script>

<div class="min-h-screen flex flex-col">
	<header class="navbar bg-base-100 shadow-md px-4">
		<div class="flex-1">
			<a class="btn btn-ghost text-xl font-bold" href="/">
				<span class="text-error">mini-redis</span>
				<span class="text-base-content/60 text-sm font-mono">rust · studio</span>
			</a>
		</div>
		<nav class="flex-none">
			<ul class="menu menu-horizontal gap-1">
				{#each links as link (link.href)}
					<li>
						<a
							href={link.href}
							class:menu-active={$page.url.pathname === link.href ||
								(link.href !== '/' && $page.url.pathname.startsWith(link.href))}
						>
							{link.label}
						</a>
					</li>
				{/each}
			</ul>
		</nav>
	</header>

	<main class="flex-1 container mx-auto px-4 py-6 max-w-6xl">
		{@render children()}
	</main>

	<footer class="footer footer-center p-4 bg-base-300 text-base-content/70 text-xs">
		<aside>
			<p>
				cs01-mini-redis-rust · M2.2 SvelteKit UI ·
				<a class="link" href="https://github.com/Hakureirm/adsd-in-practice" target="_blank"
					rel="noreferrer">source</a
				>
			</p>
		</aside>
	</footer>
</div>
