<script lang="ts">
	import { browser } from '$app/environment';
	import { parseStatsLine } from '$lib/api/sse';
	import type { StatsEvent } from '$lib/api/types';
	import { apiPath } from '$lib/desktop';
	import { formatBytes, formatUptime } from '$lib/format';

	// $state for the latest stats frame. Null means "haven't received
	// anything yet" (initial mount, or backend not running).
	let stats = $state<StatsEvent | null>(null);
	let connectionError = $state<string | null>(null);
	let lastUpdate = $state<number | null>(null);

	// Open the EventSource inside an $effect so it ties to component
	// lifecycle (closes on unmount automatically). `browser` guards
	// against any pre-render path (SSR is disabled, but defensive).
	$effect(() => {
		if (!browser) return;
		const es = new EventSource(apiPath('/api/stats'));

		es.addEventListener('stats', (event) => {
			const parsed = parseStatsLine(event.data);
			if (parsed !== null) {
				stats = parsed;
				lastUpdate = Date.now();
				connectionError = null;
			}
		});

		es.addEventListener('error', () => {
			// EventSource auto-reconnects; surface a banner if we
			// haven't received a frame in a while.
			connectionError = 'SSE 连接异常 — 重连中…';
		});

		return () => es.close();
	});
</script>

<svelte:head>
	<title>Dashboard · mini-redis-rust</title>
</svelte:head>

<section class="space-y-6">
	<header class="flex items-baseline justify-between">
		<h1 class="text-2xl font-bold">Dashboard</h1>
		<p class="text-sm text-base-content/60 font-mono">
			{#if lastUpdate !== null}
				最近更新 {new Date(lastUpdate).toLocaleTimeString()}
			{:else}
				等待数据…
			{/if}
		</p>
	</header>

	{#if connectionError}
		<div class="alert alert-warning">
			<span>{connectionError}</span>
		</div>
	{/if}

	<div class="stats stats-vertical lg:stats-horizontal shadow w-full bg-base-100">
		<div class="stat">
			<div class="stat-title">活跃连接</div>
			<div class="stat-value text-primary">
				{stats?.connections_active ?? '—'}
			</div>
			<div class="stat-desc">connections_active</div>
		</div>

		<div class="stat">
			<div class="stat-title">命令总数</div>
			<div class="stat-value">
				{stats?.commands_total?.toLocaleString() ?? '—'}
			</div>
			<div class="stat-desc">commands_total</div>
		</div>

		<div class="stat">
			<div class="stat-title">Key 总数</div>
			<div class="stat-value text-secondary">
				{stats?.keys_active?.toLocaleString() ?? '—'}
			</div>
			<div class="stat-desc">keys_active</div>
		</div>
	</div>

	<div class="stats stats-vertical lg:stats-horizontal shadow w-full bg-base-100">
		<div class="stat">
			<div class="stat-title">值占用内存</div>
			<div class="stat-value text-accent">
				{stats !== null ? formatBytes(stats.mem_value_bytes) : '—'}
			</div>
			<div class="stat-desc">
				mem_value_bytes
				{#if stats !== null}
					· raw {stats.mem_value_bytes.toLocaleString()} B
				{/if}
			</div>
		</div>

		<div class="stat">
			<div class="stat-title">运行时间</div>
			<div class="stat-value">
				{stats !== null ? formatUptime(stats.uptime_secs) : '—'}
			</div>
			<div class="stat-desc">uptime_secs</div>
		</div>
	</div>

	<details class="collapse collapse-arrow bg-base-100">
		<summary class="collapse-title text-sm font-mono">原始 SSE 帧 (debug)</summary>
		<div class="collapse-content">
			<pre class="text-xs overflow-x-auto"><code
					>{stats !== null ? JSON.stringify(stats, null, 2) : '(no frame yet)'}</code
				></pre>
		</div>
	</details>
</section>
