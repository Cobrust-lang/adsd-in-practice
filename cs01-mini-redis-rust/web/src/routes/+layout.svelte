<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { readDesktopBackendStatus } from '$lib/desktop';
	import type { DesktopBackendStatus } from '$lib/desktop';
	import '../app.css';

	let { children } = $props();
	let desktopStatus = $state<DesktopBackendStatus | null>(null);

	const links = [
		{ href: '/', label: 'Dashboard' },
		{ href: '/keys', label: 'Keys' },
		{ href: '/pubsub', label: 'Pub/Sub' }
	] as const;

	$effect(() => {
		if (!browser) return;
		let cancelled = false;
		async function refresh() {
			const status = await readDesktopBackendStatus();
			if (!cancelled) {
				desktopStatus = status;
			}
		}
		refresh();
		const interval = window.setInterval(refresh, 1000);
		return () => {
			cancelled = true;
			window.clearInterval(interval);
		};
	});
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

	<main class="flex-1 container mx-auto px-4 py-6 max-w-6xl space-y-4">
		{#if desktopStatus !== null}
			<div
				class="alert"
				class:alert-success={desktopStatus.kind === 'running'}
				class:alert-warning={desktopStatus.kind === 'starting' || desktopStatus.kind === 'stopped'}
				class:alert-error={desktopStatus.kind === 'failed'}
			>
				<span>
					<strong>Tauri sidecar:</strong> {desktopStatus.message}
					<span class="font-mono text-xs">
						RESP 127.0.0.1:{desktopStatus.resp_port} · HTTP/SSE 127.0.0.1:{desktopStatus.http_port}
					</span>
				</span>
			</div>
		{/if}
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
