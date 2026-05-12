<script lang="ts">
	// Wave M3.1 (ADR-0009) — read-only Pub/Sub dashboard.
	// Subscribes to /api/pubsub SSE and renders the channel→subscriber
	// table. Write actions (SUBSCRIBE / PUBLISH) intentionally NOT in
	// the UI for M3.1: the ADR §Q8 defers the web→RESP bridge to M4.
	// The banner below tells users to use a real RESP client.
	import { browser } from '$app/environment';
	import { parsePubsubLine } from '$lib/api/sse';
	import type { PubsubSnapshot } from '$lib/api/types';
	import { apiPath } from '$lib/desktop';

	let snapshot = $state<PubsubSnapshot | null>(null);
	let lastUpdate = $state<number | null>(null);
	let connectionError = $state<string | null>(null);

	$effect(() => {
		if (!browser) return;
		const es = new EventSource(apiPath('/api/pubsub'));

		es.addEventListener('pubsub', (event) => {
			const parsed = parsePubsubLine(event.data);
			if (parsed !== null) {
				snapshot = parsed;
				lastUpdate = Date.now();
				connectionError = null;
			}
		});

		es.addEventListener('error', () => {
			connectionError = 'SSE 连接异常 — 重连中…';
		});

		return () => es.close();
	});
</script>

<svelte:head>
	<title>Pub/Sub · mini-redis-rust</title>
</svelte:head>

<section class="space-y-4">
	<header class="flex items-baseline justify-between">
		<h1 class="text-2xl font-bold">Pub/Sub</h1>
		<p class="text-sm text-base-content/60 font-mono">
			{#if lastUpdate !== null}
				最近更新 {new Date(lastUpdate).toLocaleTimeString()} · {snapshot?.channels.length ?? 0} 频道
			{:else}
				等待数据…
			{/if}
		</p>
	</header>

	<div class="alert alert-info">
		<span>
			<strong>This dashboard is read-only.</strong>
			To subscribe / publish, use a RESP client like
			<code class="badge badge-ghost font-mono">redis-cli -p 6380</code>.
			UI write support arrives in Wave M4.
		</span>
	</div>

	{#if connectionError}
		<div class="alert alert-warning">
			<span>{connectionError}</span>
		</div>
	{/if}

	<div class="overflow-x-auto bg-base-100 rounded-box shadow">
		<table class="table table-zebra">
			<thead>
				<tr>
					<th class="w-2/3">Channel</th>
					<th>Subscribers</th>
				</tr>
			</thead>
			<tbody>
				{#if snapshot === null}
					<tr>
						<td colspan="2" class="text-center text-base-content/50 py-8">
							等待第一帧 SSE…
						</td>
					</tr>
				{:else if snapshot.channels.length === 0}
					<tr>
						<td colspan="2" class="text-center text-base-content/50 py-8">
							没有活跃频道。用 <code class="font-mono">SUBSCRIBE</code> 来创建一个。
						</td>
					</tr>
				{:else}
					{#each snapshot.channels as ch (ch.name)}
						<tr>
							<td class="font-mono break-all">{ch.name}</td>
							<td class="font-mono">{ch.subscribers}</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
</section>
