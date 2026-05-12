<script lang="ts">
	import { browser } from '$app/environment';
	import { parseKeysLine } from '$lib/api/sse';
	import type { KeyInfo } from '$lib/api/types';
	import { apiPath } from '$lib/desktop';
	import { formatTtl } from '$lib/format';

	let keys = $state<KeyInfo[]>([]);
	let lastUpdate = $state<number | null>(null);
	let connectionError = $state<string | null>(null);

	// Backend caps the snapshot at 100 keys (KEYS_SAMPLE_LIMIT in
	// http.rs). We mirror that constant here so the banner stays in
	// sync if the backend ever raises the cap.
	const SAMPLE_LIMIT = 100;

	$effect(() => {
		if (!browser) return;
		const es = new EventSource(apiPath('/api/keys'));

		es.addEventListener('keys', (event) => {
			const parsed = parseKeysLine(event.data);
			if (parsed !== null) {
				keys = parsed;
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
	<title>Keys · mini-redis-rust</title>
</svelte:head>

<section class="space-y-4">
	<header class="flex items-baseline justify-between">
		<h1 class="text-2xl font-bold">Keys</h1>
		<p class="text-sm text-base-content/60 font-mono">
			{#if lastUpdate !== null}
				最近更新 {new Date(lastUpdate).toLocaleTimeString()} · {keys.length} 行
			{:else}
				等待数据…
			{/if}
		</p>
	</header>

	<div class="alert alert-info">
		<span>
			Showing up to <strong>{SAMPLE_LIMIT}</strong> keys; use SCAN in M3 for the full keyspace.
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
					<th class="w-1/2">Key</th>
					<th>Type</th>
					<th>TTL</th>
				</tr>
			</thead>
			<tbody>
				{#if keys.length === 0}
					<tr>
						<td colspan="3" class="text-center text-base-content/50 py-8">
							{#if lastUpdate === null}
								等待第一帧 SSE…
							{:else}
								keyspace 为空
							{/if}
						</td>
					</tr>
				{:else}
					{#each keys as info (info.key)}
						<tr>
							<td class="font-mono break-all">{info.key}</td>
							<td>
								<span class="badge badge-outline">{info.type}</span>
							</td>
							<td class="font-mono">{formatTtl(info.ttl_secs)}</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
</section>
